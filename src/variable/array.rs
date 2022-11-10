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
    sync::{CommitFuture, ValueGuard, VarActive, VarSync},
};
use crate::raw::{self, variable::Action};

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

    unsafe fn value(&self) -> &FlatVec<T> {
        let cap = self.max_len();
        &*(slice::from_raw_parts(self.raw().value_ptr() as *const u8, cap) as *const [u8]
            as *const [T] as *const FlatVec<T>)
    }
    unsafe fn value_mut(&mut self) -> &mut FlatVec<T> {
        let cap = self.max_len();
        &mut *(slice::from_raw_parts_mut(self.raw_mut().value_mut_ptr() as *mut u8, cap)
            as *mut [u8] as *mut [T] as *mut FlatVec<T>)
    }
}

impl<'a, T: Copy, const R: bool, const A: bool> ValueGuard<'a, ArrayVariable<T, R, true, A>> {
    pub fn write(mut self) -> WriteGuard<'a, T, R, A> {
        unsafe { self.owner_mut().value_mut().clear() };
        WriteGuard { guard: self }
    }
}

impl<'a, T: Copy, const W: bool, const A: bool> ValueGuard<'a, ArrayVariable<T, true, W, A>> {
    pub fn read(self) -> ReadGuard<'a, T, W, A> {
        ReadGuard { guard: self }
    }
}

#[must_use]
pub struct WriteGuard<'a, T: Copy, const R: bool, const A: bool> {
    guard: ValueGuard<'a, ArrayVariable<T, R, true, A>>,
}

impl<'a, T: Copy, const R: bool, const A: bool> WriteGuard<'a, T, R, A> {
    pub fn commit(self) -> CommitFuture<'a, ArrayVariable<T, R, true, A>> {
        self.guard.commit(Action::Write)
    }
    pub fn discard(self) -> CommitFuture<'a, ArrayVariable<T, R, true, A>> {
        self.guard.discard()
    }
}

impl<'a, T: Copy, const R: bool, const A: bool> Deref for WriteGuard<'a, T, R, A> {
    type Target = FlatVec<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { self.guard.owner().value() }
    }
}
impl<'a, T: Copy, const R: bool, const A: bool> DerefMut for WriteGuard<'a, T, R, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.guard.owner_mut().value_mut() }
    }
}

#[must_use]
pub struct ReadGuard<'a, T: Copy, const W: bool, const A: bool> {
    guard: ValueGuard<'a, ArrayVariable<T, true, W, A>>,
}

impl<'a, T: Copy, const W: bool, const A: bool> ReadGuard<'a, T, W, A> {
    pub fn close(self) -> CommitFuture<'a, ArrayVariable<T, true, W, A>> {
        self.guard.commit(Action::Read)
    }
}

impl<'a, T: Copy, const W: bool, const A: bool> Deref for ReadGuard<'a, T, W, A> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { self.guard.owner().value() }
    }
}
