use std::marker::PhantomData;

use super::{
    any::{AnyVariable, Var},
    sync::{CommitFuture, ValueGuard, VarActive, VarSync},
};
use crate::raw::{self, variable::Action};

#[repr(transparent)]
pub struct Variable<T: Copy, const R: bool, const W: bool, const A: bool> {
    any: AnyVariable,
    _phantom: PhantomData<T>,
}

impl<T: Copy, const R: bool, const W: bool, const A: bool> Var for Variable<T, R, W, A> {
    fn raw(&self) -> &raw::Variable {
        self.any.raw()
    }
    fn raw_mut(&mut self) -> &mut raw::Variable {
        self.any.raw_mut()
    }
}

impl<T: Copy, const R: bool, const W: bool, const A: bool> VarSync for Variable<T, R, W, A> {}

impl<T: Copy, const R: bool, const W: bool> VarActive for Variable<T, R, W, true> {}

impl<T: Copy, const R: bool, const W: bool, const A: bool> Variable<T, R, W, A> {
    pub(crate) unsafe fn from_any(any: AnyVariable) -> Self {
        Self {
            any,
            _phantom: PhantomData,
        }
    }

    unsafe fn value(&self) -> &T {
        &*(self.raw().value().data as *const T)
    }
    unsafe fn value_mut(&mut self) -> &mut T {
        &mut *(self.raw_mut().value_mut().data as *mut T)
    }
}

impl<'a, T: Copy, const R: bool, const A: bool> ValueGuard<'a, Variable<T, R, true, A>> {
    pub fn write(mut self, value: T) -> CommitFuture<'a, Variable<T, R, true, A>> {
        *unsafe { self.owner_mut().value_mut() } = value;
        self.commit(Action::Write)
    }
}

impl<'a, T: Copy, const W: bool, const A: bool> ValueGuard<'a, Variable<T, true, W, A>> {
    pub async fn read(self) -> T {
        let value = *unsafe { self.owner().value() };
        self.commit(Action::Read).await;
        value
    }
}
