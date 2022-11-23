use stavec::GenericVec;
use std::{
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    slice,
};

pub type FlatVec<T> = GenericVec<T, [MaybeUninit<T>]>;

use super::{
    any::{AnyVariable, Var},
    sync::{Commit, ValueGuard, VarActive, VarSync},
};
use crate::raw;

#[repr(transparent)]
pub struct ArrayVariable<T: Copy, const R: bool, const W: bool, const A: bool> {
    any: AnyVariable,
    _phantom: PhantomData<T>,
}

impl<T: Copy, const R: bool, const W: bool, const A: bool> Var for ArrayVariable<T, R, W, A> {
    fn raw(&self) -> &raw::Variable {
        self.any.raw()
    }
    fn raw_mut(&mut self) -> &mut raw::Variable {
        self.any.raw_mut()
    }
}

impl<T: Copy, const R: bool, const W: bool, const A: bool> VarSync for ArrayVariable<T, R, W, A> {}

impl<T: Copy, const R: bool, const W: bool> VarActive for ArrayVariable<T, R, W, true> {}

impl<T: Copy, const R: bool, const W: bool, const A: bool> ArrayVariable<T, R, W, A> {
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
        &*(slice::from_raw_parts(self.raw().value_ptr() as *const u8, cap) as *const [u8]
            as *const [T] as *const FlatVec<T>)
    }
}
impl<T: Copy, const R: bool, const A: bool> ArrayVariable<T, R, true, A> {
    unsafe fn value_mut(&mut self) -> &mut FlatVec<T> {
        let cap = self.max_len();
        &mut *(slice::from_raw_parts_mut(self.raw_mut().value_mut_ptr() as *mut u8, cap)
            as *mut [u8] as *mut [T] as *mut FlatVec<T>)
    }
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Deref
    for ValueGuard<'a, ArrayVariable<T, R, W, A>>
{
    type Target = FlatVec<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { self.owner().value_ref() }
    }
}
impl<'a, T: Copy, const R: bool, const A: bool> DerefMut
    for ValueGuard<'a, ArrayVariable<T, R, true, A>>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.owner_mut().value_mut() }
    }
}

impl<'a, T: Copy, const R: bool, const A: bool> ValueGuard<'a, ArrayVariable<T, R, true, A>> {
    pub fn write_from<I: IntoIterator<Item = T>>(
        mut self,
        iter: I,
    ) -> Commit<'a, ArrayVariable<T, R, true, A>> {
        self.clear();
        self.extend(iter);
        self.accept()
    }
    pub fn write_from_slice(mut self, slice: &[T]) -> Commit<'a, ArrayVariable<T, R, true, A>> {
        self.clear();
        self.extend_from_slice(slice);
        self.accept()
    }
    pub fn write_from_iter<I: Iterator<Item = T>>(
        mut self,
        iter: I,
    ) -> Commit<'a, ArrayVariable<T, R, true, A>> {
        self.clear();
        self.extend_from_iter(iter);
        self.accept()
    }
}

impl<'a, T: Copy, const W: bool, const A: bool> ValueGuard<'a, ArrayVariable<T, true, W, A>> {
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
