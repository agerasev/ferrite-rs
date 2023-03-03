mod array;
mod scalar;

pub use array::FlatVec;
pub use scalar::Type;

use crate::{
    variable::{Stage, Status},
    Variable,
};
use derive_more::{Deref, DerefMut};
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

pub trait Value: Sync + 'static {}
impl<V: ?Sized + Sync + 'static> Value for V {}

#[repr(transparent)]
#[derive(Deref, DerefMut)]
pub struct TypedVariable<V: Value + ?Sized> {
    #[deref]
    #[deref_mut]
    base: Variable,
    _phantom: PhantomData<V>,
}

impl<V: Value + ?Sized> TypedVariable<V> {
    pub(crate) unsafe fn new_unchecked(base: Variable) -> Self {
        Self {
            base,
            _phantom: PhantomData,
        }
    }
}

impl<V: Value + ?Sized> TypedVariable<V> {
    /// Passively wait for variable being processed.
    pub fn wait(&mut self) -> Acquire<'_, V> {
        Acquire {
            owner: Some(self),
            request: false,
        }
    }
    /// Actively request variable processing.
    pub fn request(&mut self) -> Acquire<'_, V> {
        Acquire {
            owner: Some(self),
            request: true,
        }
    }
}

#[must_use]
pub struct Acquire<'a, V: Value + ?Sized> {
    owner: Option<&'a mut TypedVariable<V>>,
    request: bool,
}

impl<'a, V: Value + ?Sized> Unpin for Acquire<'a, V> {}

impl<'a, V: Value + ?Sized> Future for Acquire<'a, V> {
    type Output = ValueGuard<'a, V>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let owner = self.owner.take().unwrap();
        let state = owner.state();
        state.set_waker(cx.waker());
        match state.stage() {
            Stage::Idle => {
                if self.request {
                    unsafe { owner.lock().request_proc() };
                }
            }
            Stage::Requested => (),
            Stage::Processing => {
                return Poll::Ready(ValueGuard::new(owner));
            }
            _ => (),
        }
        assert!(self.owner.replace(owner).is_none());
        Poll::Pending
    }
}

#[must_use]
pub struct ValueGuard<'a, V: Value + ?Sized> {
    owner: Option<&'a mut TypedVariable<V>>,
}

impl<'a, V: Value + ?Sized> ValueGuard<'a, V> {
    fn new(owner: &'a mut TypedVariable<V>) -> Self {
        Self { owner: Some(owner) }
    }

    pub(crate) fn owner(&self) -> &TypedVariable<V> {
        self.owner.as_ref().unwrap()
    }
    pub(crate) fn owner_mut(&mut self) -> &mut TypedVariable<V> {
        self.owner.as_mut().unwrap()
    }

    unsafe fn commit_in_place(&mut self, status: Status<'_>) {
        let owner = self.owner_mut();
        assert_eq!(owner.state().stage(), Stage::Processing);
        owner.lock().commit(status);
    }
    pub(crate) fn commit(mut self, status: Status<'_>) -> Commit<'a, V> {
        unsafe { self.commit_in_place(status) };
        Commit {
            owner: self.owner.take().unwrap(),
        }
    }

    /// Successfully complete processing and commit value (if needed).
    pub fn accept(self) -> Commit<'a, V> {
        self.commit(Status::Ok(()))
    }
    /// Report that error occured during value processing.
    ///
    /// *Value updates (if any) will be commited anyway.*
    pub fn reject(self, message: &str) -> Commit<'a, V> {
        self.commit(Status::Err(message))
    }
}

impl<'a, V: Value + ?Sized> Drop for ValueGuard<'a, V> {
    fn drop(&mut self) {
        if self.owner.is_some() {
            unsafe { self.commit_in_place(Status::Err("Unhandled error")) };
        }
    }
}

#[must_use]
pub struct Commit<'a, V: Value + ?Sized> {
    owner: &'a mut TypedVariable<V>,
}

impl<'a, V: Value + ?Sized> Unpin for Commit<'a, V> {}

impl<'a, V: Value + ?Sized> Future for Commit<'a, V> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.owner.state();
        state.set_waker(cx.waker());
        match state.stage() {
            Stage::Commited => Poll::Pending,
            Stage::Idle | Stage::Processing => Poll::Ready(()),
            _ => unreachable!(),
        }
    }
}
