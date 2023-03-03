use super::{Commit, TypedVariable, ValueGuard};
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

impl<'a, T: Type> Deref for ValueGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.owner().value_ref() }
    }
}
impl<'a, T: Type> DerefMut for ValueGuard<'a, T> {
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

impl<'a, T: Type> ValueGuard<'a, T> {
    pub async fn read(self) -> T {
        let value = *self;
        self.accept().await;
        value
    }
}
