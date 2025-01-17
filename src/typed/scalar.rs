use super::{Commit, TypedVariable, ValueGuard};
use futures::stream::{self, FusedStream, StreamExt};
use std::ops::{Deref, DerefMut};

pub trait Type: Copy + Send + Sync + 'static {}
impl<V: Copy + Send + Sync + 'static> Type for V {}

impl<T: Type> TypedVariable<T> {
    unsafe fn value_ref(&self) -> &T {
        &*(self.value_ptr() as *const T)
    }
    unsafe fn value_mut(&mut self) -> &mut T {
        &mut *(self.value_ptr() as *mut T)
    }
}

impl<T: Type> Deref for ValueGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.owner().value_ref() }
    }
}
impl<T: Type> DerefMut for ValueGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.owner_mut().value_mut() }
    }
}

impl<'a, T: Type> ValueGuard<'a, T> {
    pub fn write(mut self, value: T) -> Commit<'a, T> {
        *unsafe { self.owner_mut().value_mut() } = value;
        self.accept()
    }
}

impl<T: Type> ValueGuard<'_, T> {
    pub async fn read(self) -> T {
        let value = *self;
        self.accept().await;
        value
    }
}

impl<T: Type> TypedVariable<T> {
    pub fn into_stream(self) -> impl FusedStream<Item = T> {
        stream::unfold(self, move |mut this| async move {
            Some((this.wait().await.read().await, this))
        })
        .fuse()
    }
}
