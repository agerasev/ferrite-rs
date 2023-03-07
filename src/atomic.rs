use crate::{
    typed::Type,
    variable::{LockedVariable, Stage, Status},
    TypedVariable,
};
use atomic::Atomic;
use futures::task::{waker_ref, ArcWake};
use std::{
    fmt::Debug,
    sync::{atomic::Ordering, Arc},
};

pub struct AtomicVariable<T: Type> {
    variable: TypedVariable<T>,
    value: Atomic<T>,
    update: Atomic<bool>,
}

impl<T: Type + Default> AtomicVariable<T> {
    pub fn new(variable: TypedVariable<T>) -> Arc<Self> {
        Arc::new(Self {
            variable,
            value: Atomic::default(),
            update: Atomic::new(false),
        })
    }
}

impl<T: Type + Debug> AtomicVariable<T> {
    fn notify(self: &Arc<Self>, locked: &mut LockedVariable<'_>) {
        let state = locked.state();
        state.set_waker(&waker_ref(self));
        match state.stage() {
            Stage::Idle => {
                if self.update.load(Ordering::Acquire) {
                    unsafe { locked.request_proc() }
                }
            }
            Stage::Requested => (),
            Stage::Processing => unsafe {
                if self.update.swap(false, Ordering::AcqRel) {
                    *(locked.value_ptr() as *mut T) = self.value.load(Ordering::Acquire);
                } else {
                    self.value
                        .store(*(locked.value_ptr() as *const T), Ordering::Release);
                }
                locked.commit(Status::Ok(()));
            },
            Stage::Commited => (),
        }
    }
    pub fn store(self: &Arc<Self>, value: T) {
        self.value.store(value, Ordering::Release);
        self.update.store(true, Ordering::Release);
        self.notify(&mut self.variable.lock());
    }
    pub fn load(self: &Arc<Self>) -> T {
        self.value.load(Ordering::Acquire)
    }
}

impl<T: Type + Debug> ArcWake for AtomicVariable<T> {
    fn wake_by_ref(this: &Arc<Self>) {
        // Variable is already locked when waker is called.
        let mut not_locked = unsafe { LockedVariable::without_lock(&this.variable) };
        this.notify(&mut not_locked);
    }
}
