use std::{
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use super::WriteVariable;
use crate::raw::{self, variable::ProcState};

#[repr(transparent)]
pub struct ReadVariable<T: Copy> {
    raw: raw::Variable,
    _phantom: PhantomData<T>,
}

impl<T: Copy> ReadVariable<T> {
    pub(crate) fn from_raw(raw: raw::Variable) -> Self {
        Self {
            raw,
            _phantom: PhantomData,
        }
    }

    pub fn read(&mut self) -> ReadFuture<'_, T> {
        ReadFuture {
            owner: self,
            value: None,
            complete: false,
        }
    }
}

impl<T: Copy> Deref for ReadVariable<T> {
    type Target = WriteVariable<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const _ as *const WriteVariable<T>) }
    }
}
impl<T: Copy> DerefMut for ReadVariable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self as *mut _ as *mut WriteVariable<T>) }
    }
}

pub struct ReadFuture<'a, T: Copy> {
    owner: &'a mut ReadVariable<T>,
    value: Option<T>,
    complete: bool,
}

impl<'a, T: Copy> Unpin for ReadFuture<'a, T> {}

impl<'a, T: Copy> Future for ReadFuture<'a, T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        assert!(!self.complete);
        let info = self.owner.raw.info();
        info.set_waker(cx.waker());
        match info.proc_state() {
            ProcState::Idle => unsafe { self.owner.raw.lock().request_proc() },
            ProcState::Requested => (),
            ProcState::Processing => {
                let value;
                {
                    let mut guard = self.owner.raw.lock();
                    value = unsafe { *(guard.data_ptr() as *const T) };
                    unsafe { guard.complete_proc() };
                }
                self.value = Some(value);
            }
            ProcState::Ready => (),
            ProcState::Complete => {
                unsafe { self.owner.raw.clean_proc() };
                self.complete = true;
                return Poll::Ready(self.value.take().unwrap());
            }
        }
        Poll::Pending
    }
}

impl<'a, T: Copy> Drop for ReadFuture<'a, T> {
    fn drop(&mut self) {
        log::trace!("ReadFuture::drop()");
    }
}
