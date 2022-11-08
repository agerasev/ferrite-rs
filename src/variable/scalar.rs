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
pub struct Variable<T: Copy, const R: bool, const W: bool, const A: bool> {
    raw: raw::Variable,
    _phantom: PhantomData<T>,
}

impl<T: Copy, const R: bool, const W: bool, const A: bool> Variable<T, R, W, A> {
    pub(crate) fn from_raw(raw: raw::Variable) -> Self {
        Self {
            raw,
            _phantom: PhantomData,
        }
    }

    fn value(&self) -> &T {
        unsafe { &*(self.raw.value().data as *const T) }
    }
    fn value_mut(&mut self) -> &mut T {
        unsafe { &mut *(self.raw.value_mut().data as *mut T) }
    }

    pub fn wait(&mut self) -> WaitFuture<'_, T, R, W, A> {
        WaitFuture {
            owner: Some(self),
            request: false,
        }
    }
}

impl<T: Copy, const R: bool, const W: bool> Variable<T, R, W, true> {
    pub fn request(&mut self) -> WaitFuture<'_, T, R, W, true> {
        WaitFuture {
            owner: Some(self),
            request: true,
        }
    }
}

impl<T: Copy, const R: bool, const W: bool, const A: bool> Deref for Variable<T, R, W, A> {
    type Target = AnyVariable;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const _ as *const AnyVariable) }
    }
}
impl<T: Copy, const R: bool, const W: bool, const A: bool> DerefMut for Variable<T, R, W, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self as *mut _ as *mut AnyVariable) }
    }
}

#[must_use]
pub struct WaitFuture<'a, T: Copy, const R: bool, const W: bool, const A: bool> {
    owner: Option<&'a mut Variable<T, R, W, A>>,
    request: bool,
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Unpin
    for WaitFuture<'a, T, R, W, A>
{
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Future
    for WaitFuture<'a, T, R, W, A>
{
    type Output = Value<'a, T, R, W, A>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let owner = self.owner.take().unwrap();
        let state = owner.raw.state();
        state.set_waker(cx.waker());
        match state.proc_state() {
            ProcState::Idle => {
                if self.request {
                    log::info!("{}: request", owner.name());
                    unsafe { owner.raw.lock().request_proc() };
                }
            }
            ProcState::Requested => (),
            ProcState::Processing => {
                log::info!("{}: processing", owner.name());
                return Poll::Ready(Value { owner });
            }
            _ => unreachable!(),
        }
        assert!(self.owner.replace(owner).is_none());
        Poll::Pending
    }
}

pub struct Value<'a, T: Copy, const R: bool, const W: bool, const A: bool> {
    owner: &'a mut Variable<T, R, W, A>,
}

impl<'a, T: Copy, const W: bool, const A: bool> Value<'a, T, true, W, A> {
    pub fn read(&mut self) -> ReadFuture<'_, T, W, A> {
        ReadFuture {
            owner: self.owner,
            value: None,
        }
    }
}

impl<'a, T: Copy, const R: bool, const A: bool> Value<'a, T, R, true, A> {
    pub fn write(&mut self, value: T) -> WriteFuture<'_, T, R, A> {
        WriteFuture {
            owner: self.owner,
            value: Some(value),
        }
    }
}

#[must_use]
pub struct ReadFuture<'a, T: Copy, const W: bool, const A: bool> {
    owner: &'a mut Variable<T, true, W, A>,
    value: Option<T>,
}

impl<'a, T: Copy, const W: bool, const A: bool> Unpin for ReadFuture<'a, T, W, A> {}

impl<'a, T: Copy, const W: bool, const A: bool> Future for ReadFuture<'a, T, W, A> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.owner.raw.state();
        state.set_waker(cx.waker());
        match state.proc_state() {
            ProcState::Processing => {
                let value = *self.owner.value();
                unsafe { self.owner.raw.lock().complete_read() };
                assert!(self.value.replace(value).is_none());
            }
            ProcState::Ready => (),
            ProcState::Complete => {
                unsafe { self.owner.raw.clean_proc() };
                return Poll::Ready(self.value.take().unwrap());
            }
            _ => unreachable!(),
        }
        Poll::Pending
    }
}

#[must_use]
pub struct WriteFuture<'a, T: Copy, const R: bool, const A: bool> {
    owner: &'a mut Variable<T, R, true, A>,
    value: Option<T>,
}

impl<'a, T: Copy, const R: bool, const A: bool> Unpin for WriteFuture<'a, T, R, A> {}

impl<'a, T: Copy, const R: bool, const A: bool> Future for WriteFuture<'a, T, R, A> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let state = self.owner.raw.state();
        state.set_waker(cx.waker());
        match state.proc_state() {
            ProcState::Processing => {
                *self.owner.value_mut() = self.value.take().unwrap();
                unsafe { self.owner.raw.lock().complete_write() };
            }
            ProcState::Ready => (),
            ProcState::Complete => {
                unsafe { self.owner.raw.clean_proc() };
                return Poll::Ready(());
            }
            _ => unreachable!(),
        }
        Poll::Pending
    }
}
