use crate::{
    typed::Type,
    variable::{Stage, Status},
    TypedVariable,
};
use atomic::Atomic;
use futures::task::{waker_ref, ArcWake};
use std::sync::{atomic::Ordering, Arc, Mutex};

pub struct AtomicVariable<T: Type> {
    variable: Mutex<TypedVariable<T>>,
    value: Atomic<T>,
}

impl<T: Type + Default> AtomicVariable<T> {
    pub fn new(variable: TypedVariable<T>) -> Arc<Self> {
        Arc::new(Self {
            variable: Mutex::new(variable),
            value: Atomic::default(),
        })
    }
}
impl<T: Type> AtomicVariable<T> {
    fn notify(self: &Arc<Self>) {
        let mut guard = self.variable.lock().unwrap();
        let mut locked = guard.lock();
        let state = locked.state();
        state.set_waker(&waker_ref(&self));
        match state.stage() {
            Stage::Idle => unsafe { locked.request_proc() },
            Stage::Requested => (),
            Stage::Processing => unsafe {
                *(locked.value_ptr() as *mut T) = self.value.load(Ordering::Acquire);
                locked.commit(Status::Ok(()));
            },
            Stage::Commited => (),
        }
    }
    pub fn store(self: &Arc<Self>, value: T) {
        self.value.store(value, Ordering::Release);
        self.notify();
    }
}

impl<T: Type> ArcWake for AtomicVariable<T> {
    fn wake_by_ref(this: &Arc<Self>) {
        this.notify();
    }
}
