use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use super::{
    any::{AnyVariable, Var},
    sync::{Commit, ValueGuard, VarSync},
};
use crate::raw;

#[repr(transparent)]
pub struct Variable<T: Copy> {
    any: AnyVariable,
    _phantom: PhantomData<T>,
}

impl<T: Copy> Var for Variable<T> {
    fn raw(&self) -> &raw::Variable {
        self.any.raw()
    }
    fn raw_mut(&mut self) -> &mut raw::Variable {
        self.any.raw_mut()
    }
}

impl<T: Copy> VarSync for Variable<T> {}

impl<T: Copy> Variable<T> {
    pub(crate) unsafe fn from_any(any: AnyVariable) -> Self {
        Self {
            any,
            _phantom: PhantomData,
        }
    }

    unsafe fn value_ref(&self) -> &T {
        &*(self.raw().value_ptr() as *const T)
    }
}
impl<T: Copy> Variable<T> {
    unsafe fn value_mut(&mut self) -> &mut T {
        &mut *(self.raw_mut().value_mut_ptr() as *mut T)
    }
}

impl<'a, T: Copy> Deref for ValueGuard<'a, Variable<T>> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.owner().value_ref() }
    }
}
impl<'a, T: Copy> DerefMut for ValueGuard<'a, Variable<T>> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.owner_mut().value_mut() }
    }
}

impl<'a, T: Copy> ValueGuard<'a, Variable<T>> {
    pub fn write(mut self, value: T) -> Commit<'a, Variable<T>> {
        *unsafe { self.owner_mut().value_mut() } = value;
        self.accept()
    }
}

impl<'a, T: Copy> ValueGuard<'a, Variable<T>> {
    pub async fn read(self) -> T {
        let value = *self;
        self.accept().await;
        value
    }
}
