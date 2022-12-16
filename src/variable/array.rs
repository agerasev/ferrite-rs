use stavec::GenericVec;
use std::{
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr,
};

pub type FlatVec<T> = GenericVec<T, [MaybeUninit<T>]>;

use super::{
    any::{AnyVariable, Var},
    sync::{Commit, ValueGuard, VarSync},
};
use crate::raw;

#[repr(transparent)]
pub struct ArrayVariable<T: Copy> {
    any: AnyVariable,
    _phantom: PhantomData<T>,
}

impl<T: Copy> Var for ArrayVariable<T> {
    fn raw(&self) -> &raw::Variable {
        self.any.raw()
    }
    fn raw_mut(&mut self) -> &mut raw::Variable {
        self.any.raw_mut()
    }
}

impl<T: Copy> VarSync for ArrayVariable<T> {}

impl<T: Copy> ArrayVariable<T> {
    pub(crate) unsafe fn from_any(any: AnyVariable) -> Self {
        Self {
            any,
            _phantom: PhantomData,
        }
    }

    pub fn max_len(&self) -> usize {
        self.info().max_len
    }

    unsafe fn value_ref(&self) -> &FlatVec<T> {
        let cap = self.max_len();
        &*(ptr::slice_from_raw_parts(self.raw().value_ptr() as *const u8, cap) as *const [T]
            as *const FlatVec<T>)
    }
}
impl<T: Copy> ArrayVariable<T> {
    unsafe fn value_mut(&mut self) -> &mut FlatVec<T> {
        let cap = self.max_len();
        &mut *(ptr::slice_from_raw_parts_mut(self.raw_mut().value_mut_ptr() as *mut u8, cap)
            as *mut [T] as *mut FlatVec<T>)
    }
}

impl<'a, T: Copy> Deref for ValueGuard<'a, ArrayVariable<T>> {
    type Target = FlatVec<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { self.owner().value_ref() }
    }
}
impl<'a, T: Copy> DerefMut for ValueGuard<'a, ArrayVariable<T>> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.owner_mut().value_mut() }
    }
}

impl<'a, T: Copy> ValueGuard<'a, ArrayVariable<T>> {
    pub fn write_from<I: IntoIterator<Item = T>>(
        mut self,
        iter: I,
    ) -> Commit<'a, ArrayVariable<T>> {
        self.clear();
        self.extend(iter);
        self.accept()
    }
    pub fn write_from_slice(mut self, slice: &[T]) -> Commit<'a, ArrayVariable<T>> {
        self.clear();
        self.extend_from_slice(slice);
        self.accept()
    }
    pub fn write_from_iter<I: Iterator<Item = T>>(
        mut self,
        iter: I,
    ) -> Commit<'a, ArrayVariable<T>> {
        self.clear();
        self.extend_from_iter(iter);
        self.accept()
    }
}

impl<'a, T: Copy> ValueGuard<'a, ArrayVariable<T>> {
    pub async fn read_into_vec(self) -> Vec<T> {
        let res = Vec::from(self.as_ref());
        self.accept().await;
        res
    }
    pub async fn read_to_slice(self, slice: &mut [T]) -> usize {
        let len = self.len();
        slice[..len].copy_from_slice(&self);
        self.accept().await;
        len
    }
    pub async fn read_to_vec(self, vec: &mut Vec<T>) {
        vec.extend_from_slice(&self);
        self.accept().await;
    }
}
