use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::Var;
use crate::raw::variable::{Action, ProcState};

pub trait VarSync: Var {
    /// Passively wait for variable being processed.
    fn acquire(&mut self) -> Acquire<'_, Self> {
        Acquire {
            owner: Some(self),
            request: false,
        }
    }
    /// Acqure value if variable is being processed now.
    fn try_acquire(&mut self) -> Option<ValueGuard<'_, Self>> {
        if let ProcState::Processing = self.raw().state().proc_state() {
            Some(ValueGuard::new(self))
        } else {
            None
        }
    }
}

pub trait VarActive: VarSync {
    /// Request variable processing and acquire value.
    fn request(&mut self) -> Acquire<'_, Self> {
        Acquire {
            owner: Some(self),
            request: true,
        }
    }
}

#[must_use]
pub struct Acquire<'a, V: VarSync> {
    owner: Option<&'a mut V>,
    request: bool,
}

impl<'a, V: VarSync> Unpin for Acquire<'a, V> {}

impl<'a, V: VarSync> Future for Acquire<'a, V> {
    type Output = ValueGuard<'a, V>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let owner = self.owner.take().unwrap();
        let state = owner.raw().state();
        state.set_waker(cx.waker());
        match state.proc_state() {
            ProcState::Idle => {
                if self.request {
                    unsafe { owner.raw_mut().lock().request_proc() };
                }
            }
            ProcState::Requested => (),
            ProcState::Processing => {
                return Poll::Ready(ValueGuard::new(owner));
            }
            _ => (),
        }
        assert!(self.owner.replace(owner).is_none());
        Poll::Pending
    }
}

#[must_use]
pub struct ValueGuard<'a, V: VarSync> {
    owner: Option<&'a mut V>,
}

impl<'a, V: VarSync> ValueGuard<'a, V> {
    fn new(owner: &'a mut V) -> Self {
        Self { owner: Some(owner) }
    }

    pub(crate) fn owner(&self) -> &V {
        self.owner.as_ref().unwrap()
    }
    pub(crate) fn owner_mut(&mut self) -> &mut V {
        self.owner.as_mut().unwrap()
    }

    unsafe fn commit_in_place(&mut self, action: Action) {
        let raw = self.owner.as_mut().unwrap().raw_mut();
        assert_eq!(raw.state().proc_state(), ProcState::Processing);
        raw.lock().commit(action);
    }
    pub(crate) fn commit(mut self, action: Action) -> Commit<'a, V> {
        unsafe { self.commit_in_place(action) };
        Commit {
            owner: self.owner.take().unwrap(),
        }
    }
    pub fn discard(self) -> Commit<'a, V> {
        self.commit(Action::Discard)
    }
}

impl<'a, V: VarSync> Drop for ValueGuard<'a, V> {
    fn drop(&mut self) {
        if self.owner.is_some() {
            unsafe { self.commit_in_place(Action::Discard) };
        }
    }
}

#[must_use]
pub struct Commit<'a, V: VarSync> {
    owner: &'a mut V,
}

impl<'a, V: VarSync> Unpin for Commit<'a, V> {}

impl<'a, V: VarSync> Future for Commit<'a, V> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.owner.raw().state();
        state.set_waker(cx.waker());
        match state.proc_state() {
            ProcState::Commited => Poll::Pending,
            ProcState::Idle | ProcState::Processing => Poll::Ready(()),
            _ => unreachable!(),
        }
    }
}
