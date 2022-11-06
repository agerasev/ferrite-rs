use std::{
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use super::WriteArrayVariable;
use crate::raw::{self, variable::ProcState};

#[repr(transparent)]
pub struct ReadArrayVariable<T: Copy> {
    raw: raw::Variable,
    _phantom: PhantomData<T>,
}

impl<T: Copy> ReadArrayVariable<T> {
    pub(crate) fn from_raw(raw: raw::Variable) -> Self {
        Self {
            raw,
            _phantom: PhantomData,
        }
    }

    pub fn read_in_place(&mut self) -> ReadInPlaceFuture<'_, T> {
        ReadInPlaceFuture { owner: Some(self) }
    }

    pub async fn read_to_slice(&mut self, dst: &mut [T]) -> Option<usize> {
        let src = self.read_in_place().await;
        let res = if dst.len() >= src.len() {
            dst[..src.len()].copy_from_slice(&src);
            Some(src.len())
        } else {
            None
        };
        src.close().await;
        res
    }
}

impl<T: Copy> Deref for ReadArrayVariable<T> {
    type Target = WriteArrayVariable<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const _ as *const WriteArrayVariable<T>) }
    }
}
impl<T: Copy> DerefMut for ReadArrayVariable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self as *mut _ as *mut WriteArrayVariable<T>) }
    }
}

pub struct ReadInPlaceFuture<'a, T: Copy> {
    owner: Option<&'a mut ReadArrayVariable<T>>,
}

impl<'a, T: Copy> Unpin for ReadInPlaceFuture<'a, T> {}

impl<'a, T: Copy> Future for ReadInPlaceFuture<'a, T> {
    type Output = ReadArrayGuard<'a, T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let owner = self.owner.take().unwrap();
        let info = owner.raw.info();
        info.set_waker(cx.waker());
        match info.proc_state() {
            ProcState::Idle => unsafe { owner.raw.lock().request_proc() },
            ProcState::Requested => (),
            ProcState::Processing => return Poll::Ready(ReadArrayGuard::new(owner)),
            _ => unreachable!(),
        }
        self.owner.replace(owner);
        Poll::Pending
    }
}

pub struct ReadArrayGuard<'a, T: Copy> {
    owner: Option<&'a mut ReadArrayVariable<T>>,
}

impl<'a, T: Copy> ReadArrayGuard<'a, T> {
    fn new(owner: &'a mut ReadArrayVariable<T>) -> Self {
        unsafe { owner.raw.get_unprotected().lock() };
        Self { owner: Some(owner) }
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe {
            let raw_unprotected = self.owner.as_ref().unwrap().raw.get_unprotected();
            std::slice::from_raw_parts(
                raw_unprotected.data_ptr() as *const T,
                raw_unprotected.array_len(),
            )
        }
    }

    pub fn close(mut self) -> CloseArrayFuture<'a, T> {
        let owner = self.owner.take().unwrap();
        unsafe {
            let raw_unprotected = owner.raw.get_unprotected_mut();
            raw_unprotected.complete_proc();
            raw_unprotected.unlock();
        }
        CloseArrayFuture { owner: Some(owner) }
    }
}

impl<'a, T: Copy> Deref for ReadArrayGuard<'a, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<'a, T: Copy> Drop for ReadArrayGuard<'a, T> {
    fn drop(&mut self) {
        if let Some(_owner) = self.owner.take() {
            panic!("ReadArrayGuard must be explicitly closed");
        }
    }
}

pub struct CloseArrayFuture<'a, T: Copy> {
    owner: Option<&'a mut ReadArrayVariable<T>>,
}

impl<'a, T: Copy> Unpin for CloseArrayFuture<'a, T> {}

impl<'a, T: Copy> Future for CloseArrayFuture<'a, T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let owner = self.owner.take().unwrap();
        let info = owner.raw.info();
        info.set_waker(cx.waker());
        match info.proc_state() {
            ProcState::Ready => (),
            ProcState::Complete => {
                unsafe { owner.raw.clean_proc() };
                return Poll::Ready(());
            }
            _ => unreachable!(),
        }
        self.owner.replace(owner);
        Poll::Pending
    }
}
