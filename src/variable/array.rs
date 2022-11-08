use std::{
    future::Future,
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    pin::Pin,
    slice,
    task::{Context, Poll},
};

use super::AnyVariable;
use crate::raw::{self, variable::ProcState};

#[repr(transparent)]
pub struct ArrayVariable<T: Copy, const R: bool, const W: bool, const A: bool> {
    raw: raw::Variable,
    _phantom: PhantomData<T>,
}

impl<T: Copy, const R: bool, const W: bool, const A: bool> ArrayVariable<T, R, W, A> {
    pub(crate) fn from_raw(raw: raw::Variable) -> Self {
        Self {
            raw,
            _phantom: PhantomData,
        }
    }

    pub fn max_len(&self) -> usize {
        if let VariableType::Array { max_len, .. } = self.data_type() {
            max_len
        } else {
            unreachable!()
        }
    }

    pub fn init_in_place(&mut self) -> InitInPlaceFuture<'_, T> {
        InitInPlaceFuture { owner: Some(self) }
    }

    pub async fn write_from_slice(&mut self, src: &[T]) {
        assert!(src.len() <= self.max_len());
        let mut guard = self.init_in_place().await;
        let dst_uninit = guard.as_uninit_slice();
        let src_uninit =
            unsafe { slice::from_raw_parts(src.as_ptr() as *const MaybeUninit<T>, src.len()) };
        dst_uninit[..src.len()].copy_from_slice(src_uninit);
        guard.set_len(src.len());
        guard.write().await;
    }
}

impl<T: Copy, const R: bool, const W: bool, const A: bool> Deref for ArrayVariable<T, R, W, A> {
    type Target = AnyVariable;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const _ as *const AnyVariable) }
    }
}
impl<T: Copy, const R: bool, const W: bool, const A: bool> DerefMut for ArrayVariable<T, R, W, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self as *mut _ as *mut AnyVariable) }
    }
}

pub struct ReadInPlaceFuture<'a, T: Copy, const R: bool, const W: bool, const A: bool> {
    owner: Option<&'a mut ReadArrayVariable<T>>,
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Unpin for ReadInPlaceFuture<'a, T> {}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Future for ReadInPlaceFuture<'a, T> {
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

pub struct ReadArrayGuard<'a, T: Copy, const R: bool, const W: bool, const A: bool> {
    owner: Option<&'a mut ReadArrayVariable<T>>,
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> ReadArrayGuard<'a, T> {
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

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Deref for ReadArrayGuard<'a, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Drop for ReadArrayGuard<'a, T> {
    fn drop(&mut self) {
        if let Some(_owner) = self.owner.take() {
            panic!("ReadArrayGuard must be explicitly closed");
        }
    }
}

pub struct CloseArrayFuture<'a, T: Copy, const R: bool, const W: bool, const A: bool> {
    owner: Option<&'a mut ReadArrayVariable<T>>,
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Unpin for CloseArrayFuture<'a, T> {}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Future for CloseArrayFuture<'a, T> {
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

pub struct InitInPlaceFuture<'a, T: Copy, const R: bool, const W: bool, const A: bool> {
    owner: Option<&'a mut ArrayVariable<T, R, W, A>>,
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Unpin for InitInPlaceFuture<'a, T> {}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Future for InitInPlaceFuture<'a, T> {
    type Output = WriteArrayGuard<'a, T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let owner = self.owner.take().unwrap();
        let info = owner.raw.info();
        info.set_waker(cx.waker());
        match info.proc_state() {
            ProcState::Idle => unsafe { owner.raw.lock().request_proc() },
            ProcState::Requested => (),
            ProcState::Processing => return Poll::Ready(WriteArrayGuard::new(owner)),
            _ => unreachable!(),
        }
        self.owner.replace(owner);
        Poll::Pending
    }
}

#[must_use]
pub struct WriteArrayGuard<'a, T: Copy, const R: bool, const W: bool, const A: bool> {
    owner: Option<&'a mut ArrayVariable<T, R, W, A>>,
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> WriteArrayGuard<'a, T> {
    fn new(owner: &'a mut ArrayVariable<T, R, W, A>) -> Self {
        unsafe { owner.raw.get_unprotected().lock() };
        Self { owner: Some(owner) }
    }

    pub fn as_uninit_slice(&mut self) -> &mut [MaybeUninit<T>] {
        let owner = self.owner.as_ref().unwrap();
        let max_len = owner.max_len();
        unsafe {
            let raw_unprotected = owner.raw.get_unprotected();
            std::slice::from_raw_parts_mut(
                raw_unprotected.data_ptr() as *mut MaybeUninit<T>,
                max_len,
            )
        }
    }

    pub fn set_len(&mut self, new_len: usize) {
        let owner = self.owner.as_mut().unwrap();
        assert!(new_len <= owner.max_len());
        unsafe { owner.raw.get_unprotected_mut() }.array_set_len(new_len);
    }

    pub fn write(mut self) -> WriteArrayFuture<'a, T> {
        let owner = self.owner.take().unwrap();
        unsafe {
            let raw_unprotected = owner.raw.get_unprotected_mut();
            raw_unprotected.complete_proc();
            raw_unprotected.unlock();
        }
        WriteArrayFuture { owner: Some(owner) }
    }
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Drop for WriteArrayGuard<'a, T> {
    fn drop(&mut self) {
        if let Some(_owner) = self.owner.take() {
            panic!("WriteArrayGuard must be explicitly written");
        }
    }
}

pub struct WriteArrayFuture<'a, T: Copy, const R: bool, const W: bool, const A: bool> {
    owner: Option<&'a mut ArrayVariable<T, R, W, A>>,
}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Unpin for WriteArrayFuture<'a, T> {}

impl<'a, T: Copy, const R: bool, const W: bool, const A: bool> Future for WriteArrayFuture<'a, T> {
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
