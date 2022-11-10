use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::Var;
use crate::raw::variable::{Action, ProcState};

pub trait VarSync: Var {
    fn wait(&mut self) -> WaitFuture<'_, Self> {
        WaitFuture {
            owner: Some(self),
            request: false,
        }
    }
}

pub trait VarActive: VarSync {
    fn request(&mut self) -> WaitFuture<'_, Self> {
        WaitFuture {
            owner: Some(self),
            request: true,
        }
    }
}

#[must_use]
pub struct WaitFuture<'a, V: VarSync> {
    owner: Option<&'a mut V>,
    request: bool,
}

impl<'a, V: VarSync> Unpin for WaitFuture<'a, V> {}

impl<'a, V: VarSync> Future for WaitFuture<'a, V> {
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
                return Poll::Ready(ValueGuard { owner: Some(owner) });
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
    pub(crate) fn commit(mut self, action: Action) -> CommitFuture<'a, V> {
        unsafe { self.commit_in_place(action) };
        CommitFuture {
            owner: self.owner.take().unwrap(),
        }
    }
    pub fn discard(self) -> CommitFuture<'a, V> {
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
pub struct CommitFuture<'a, V: VarSync> {
    owner: &'a mut V,
}

impl<'a, V: VarSync> Unpin for CommitFuture<'a, V> {}

impl<'a, V: VarSync> Future for CommitFuture<'a, V> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.owner.raw().state();
        state.set_waker(cx.waker());
        match state.proc_state() {
            ProcState::Commited => Poll::Pending,
            ProcState::Idle => Poll::Ready(()),
            _ => unreachable!(),
        }
    }
}
