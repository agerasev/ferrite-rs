use super::{Commit, Type, TypedVariable, ValueGuard};
use stavec::GenericVec;
use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr,
};

pub type FlatVec<T> = GenericVec<[MaybeUninit<T>]>;

impl<T: Type> TypedVariable<[T]> {
    pub fn max_len(&self) -> usize {
        self.info().max_len
    }

    unsafe fn value_ref(&self) -> &FlatVec<T> {
        let cap = self.max_len();
        &*(ptr::slice_from_raw_parts(self.value_ptr() as *const u8, cap) as *const [T]
            as *const FlatVec<T>)
    }
    unsafe fn value_mut(&mut self) -> &mut FlatVec<T> {
        let cap = self.max_len();
        &mut *(ptr::slice_from_raw_parts_mut(self.value_ptr() as *mut u8, cap) as *mut [T]
            as *mut FlatVec<T>)
    }
}

impl<T: Type> Deref for ValueGuard<'_, [T]> {
    type Target = FlatVec<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { self.owner().value_ref() }
    }
}
impl<T: Type> DerefMut for ValueGuard<'_, [T]> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.owner_mut().value_mut() }
    }
}

impl<'a, T: Type> ValueGuard<'a, [T]> {
    pub fn write_from<I: IntoIterator<Item = T>>(mut self, iter: I) -> Commit<'a, [T]> {
        self.clear();
        self.extend_until_full(iter);
        self.accept()
    }
    pub fn write_from_slice(mut self, slice: &[T]) -> Commit<'a, [T]> {
        self.clear();
        let len = self.capacity().min(slice.len());
        self.push_slice(&slice[..len]).unwrap();
        self.accept()
    }
    #[deprecated = "use `write_from` instead"]
    pub fn write_from_iter<I: Iterator<Item = T>>(self, iter: I) -> Commit<'a, [T]> {
        self.write_from(iter)
    }
}

impl<T: Type> ValueGuard<'_, [T]> {
    pub async fn read_into_vec(self) -> Vec<T> {
        let res = Vec::from(self.as_slice());
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
