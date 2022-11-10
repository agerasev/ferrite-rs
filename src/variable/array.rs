use stavec::GenericVec;
use std::{
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    slice,
};

pub type InPlaceVec<'a, T> = GenericVec<T, &'a mut [MaybeUninit<T>]>;

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

    unsafe fn value(&self) -> &[T] {
        let raw = self.raw();
        let len = raw.value().len;
        slice::from_raw_parts(raw.value().data as *const T, len)
    }
    unsafe fn set_len(&mut self, len: usize) {
        assert!(len <= self.max_len());
        self.raw_mut().value_mut().len = len;
    }
}

impl<'a, T: Copy, const R: bool, const A: bool> ValueGuard<'a, ArrayVariable<T, R, true, A>> {
    pub fn write(self) -> WriteGuard<'a, T, R, A> {
        WriteGuard::new(self)
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
    value: InPlaceVec<'a, T>,
}

impl<'a, T: Copy, const R: bool, const A: bool> WriteGuard<'a, T, R, A> {
    fn new(mut guard: ValueGuard<'a, ArrayVariable<T, R, true, A>>) -> Self {
        let owner = guard.owner_mut();
        let cap = owner.max_len();
        let value = unsafe {
            InPlaceVec::from_raw_parts(
                slice::from_raw_parts_mut(
                    owner.raw_mut().value_mut().data as *mut MaybeUninit<T>,
                    cap,
                ),
                0,
            )
        };
        Self { guard, value }
    }
    pub fn commit(mut self) -> CommitFuture<'a, ArrayVariable<T, R, true, A>> {
        unsafe { self.guard.owner_mut().set_len(self.value.len()) };
        self.guard.commit(Action::Write)
    }
    pub fn discard(self) -> CommitFuture<'a, ArrayVariable<T, R, true, A>> {
        self.guard.discard()
    }
}

impl<'a, T: Copy, const R: bool, const A: bool> Deref for WriteGuard<'a, T, R, A> {
    type Target = InPlaceVec<'a, T>;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<'a, T: Copy, const R: bool, const A: bool> DerefMut for WriteGuard<'a, T, R, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
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
