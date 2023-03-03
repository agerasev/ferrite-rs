use super::{Commit, Type, TypedVariable, ValueGuard};
use stavec::GenericVec;
use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr,
};

pub type FlatVec<T> = GenericVec<T, [MaybeUninit<T>]>;

impl<T: Type> TypedVariable<[T]> {
    pub fn max_len(&self) -> usize {
        self.info().max_len
    }

    pub unsafe fn value_ref(&self) -> &FlatVec<T> {
        let cap = self.max_len();
        &*(ptr::slice_from_raw_parts(self.value_ptr() as *const u8, cap) as *const [T]
            as *const FlatVec<T>)
    }

    pub unsafe fn value_mut(&mut self) -> &mut FlatVec<T> {
        let cap = self.max_len();
        &mut *(ptr::slice_from_raw_parts_mut(self.value_ptr() as *mut u8, cap) as *mut [T]
            as *mut FlatVec<T>)
    }
}

impl<'a, T: Type> Deref for ValueGuard<'a, [T]> {
    type Target = FlatVec<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { self.owner().value_ref() }
    }
}
impl<'a, T: Type> DerefMut for ValueGuard<'a, [T]> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.owner_mut().value_mut() }
    }
}

impl<'a, T: Type> ValueGuard<'a, [T]> {
    pub fn write_from<I: IntoIterator<Item = T>>(mut self, iter: I) -> Commit<'a, [T]> {
        self.clear();
        self.extend(iter);
        self.accept()
    }
    pub fn write_from_slice(mut self, slice: &[T]) -> Commit<'a, [T]> {
        self.clear();
        self.extend_from_slice(slice);
        self.accept()
    }
    pub fn write_from_iter<I: Iterator<Item = T>>(mut self, iter: I) -> Commit<'a, [T]> {
        self.clear();
        self.extend_from_iter(iter);
        self.accept()
    }
}

impl<'a, T: Type> ValueGuard<'a, [T]> {
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
