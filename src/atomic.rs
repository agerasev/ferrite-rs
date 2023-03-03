use crate::{
    typed::Type,
    variable::{LockedVariable, Stage, Status},
    TypedVariable,
};
use atomic::Atomic;
use futures::task::{waker_ref, ArcWake};
use std::{
    fmt::Debug,
    sync::{atomic::Ordering, Arc, Mutex},
};

pub struct AtomicVariable<T: Type> {
    variable: Mutex<TypedVariable<T>>,
    value: Atomic<T>,
    updated: Atomic<bool>,
}

impl<T: Type + Default> AtomicVariable<T> {
    pub fn new(variable: TypedVariable<T>) -> Arc<Self> {
        Arc::new(Self {
            variable: Mutex::new(variable),
            value: Atomic::default(),
            updated: Atomic::new(false),
        })
    }
}
impl<T: Type + Debug> AtomicVariable<T> {
    fn notify(self: &Arc<Self>, locked: &mut LockedVariable<'_>) {
        let state = locked.state();
        state.set_waker(&waker_ref(self));
        match state.stage() {
            Stage::Idle => {
                if self.updated.load(Ordering::Acquire) {
                    unsafe { locked.request_proc() }
                }
            }
            Stage::Requested => (),
            Stage::Processing => unsafe {
                self.updated.store(false, Ordering::Release);
                *(locked.value_ptr() as *mut T) = self.value.load(Ordering::Acquire);
                locked.commit(Status::Ok(()));
            },
            Stage::Commited => (),
        }
    }
    pub fn store(self: &Arc<Self>, value: T) {
        self.value.store(value, Ordering::Release);
        self.updated.store(true, Ordering::Release);
        let mut guard = self.variable.lock().unwrap();
        self.notify(&mut guard.lock());
    }
}

impl<T: Type + Debug> ArcWake for AtomicVariable<T> {
    fn wake_by_ref(this: &Arc<Self>) {
        let mut guard = this.variable.lock().unwrap();
        // Variable is already locked when waker is called.
        let mut not_locked = unsafe { LockedVariable::without_lock(&mut guard) };
        this.notify(&mut not_locked);
    }
}
