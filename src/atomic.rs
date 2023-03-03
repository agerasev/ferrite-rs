use crate::raw::variable::{Stage, Status};

use super::{Var, VarSync, Variable};
use atomic::Atomic;
use futures::task::{waker_ref, ArcWake};
use std::{
    cell::UnsafeCell,
    sync::{atomic::Ordering, Arc},
};

pub struct AtomicVariable<T: Copy> {
    variable: Variable<T>,
    value: Atomic<T>,
}

impl<T: Copy> AtomicVariable<T> {
    pub fn new(variable: Variable<T>, value: T) -> Arc<Self> {
        let this = Arc::new(Self {
            variable: UnsafeCell::new(variable),
            value: Atomic::new(value),
        });
        this.variable.raw().state().set_waker(&waker_ref(&this));
        /*
        let handle = self_.clone();
        exec.spawn(async move {
            loop {
                handle.event.take().await;
                let value = handle.value.load(Ordering::Acquire);
                variable.request().await.write(value).await;
            }
        })?;
        Ok(self_)
        */
        this
    }
    /// # Safety
    ///
    /// Variable must be locked.
    unsafe fn variable_mut(&self) -> &mut Variable<T> {
        &mut *self.variable.get()
    }
    /// # Safety
    ///
    /// Must be called only when variable is locked.
    unsafe fn notify(self: &Arc<Self>) {
        let var = self.variable_mut();
        let raw = var.raw_mut().get_unprotected_mut();
        let state = raw.state();
        state.set_waker(&waker_ref(&self));
        match state.stage() {
            Stage::Idle => raw.request_proc(),
            Stage::Requested => (),
            Stage::Processing => {
                *var.value_mut() = self.value.load(Ordering::Acquire);
                raw.commit(Status::Ok(()));
            }
            Stage::Commited => (),
        }
    }
    pub fn write(self: &Arc<Self>, value: T) {
        self.value.store(value, Ordering::Release);
        unsafe { self.notify() };
    }
}

unsafe impl<T: Copy> Send for AtomicVariable<T> {}
unsafe impl<T: Copy> Sync for AtomicVariable<T> {}

impl<T: Copy> ArcWake for AtomicVariable<T> {
    fn wake_by_ref(this: &Arc<Self>) {
        //let state = this.variable.state();
    }
}
