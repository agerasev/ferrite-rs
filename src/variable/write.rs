use std::{
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use super::AnyVariable;
use crate::raw::{self, variable::ProcState};

#[repr(transparent)]
pub struct WriteVariable<T: Copy> {
    raw: raw::Variable,
    _phantom: PhantomData<T>,
}

impl<T: Copy> WriteVariable<T> {
    pub(crate) fn from_raw(raw: raw::Variable) -> Self {
        Self {
            raw,
            _phantom: PhantomData,
        }
    }

    pub fn write(&mut self, value: T) -> WriteFuture<'_, T> {
        WriteFuture {
            owner: self,
            value,
            complete: false,
        }
    }
}

impl<T: Copy> Deref for WriteVariable<T> {
    type Target = AnyVariable;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const _ as *const AnyVariable) }
    }
}
impl<T: Copy> DerefMut for WriteVariable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self as *mut _ as *mut AnyVariable) }
    }
}

pub struct WriteFuture<'a, T: Copy> {
    owner: &'a mut WriteVariable<T>,
    value: T,
    complete: bool,
}

impl<'a, T: Copy> Unpin for WriteFuture<'a, T> {}

impl<'a, T: Copy> Future for WriteFuture<'a, T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        assert!(!self.complete);
        let value = self.value;
        let info = self.owner.raw.info();
        info.set_waker(cx.waker());
        match info.proc_state() {
            ProcState::Idle => unsafe { self.owner.raw.lock().request_proc() },
            ProcState::Requested => (),
            ProcState::Processing => {
                let mut guard = self.owner.raw.lock();
                unsafe { *(guard.data_mut_ptr() as *mut T) = value };
                unsafe { guard.complete_proc() };
            }
            ProcState::Ready => (),
            ProcState::Complete => {
                unsafe { self.owner.raw.clean_proc() };
                self.complete = true;
                return Poll::Ready(());
            }
        }
        Poll::Pending
    }
}
