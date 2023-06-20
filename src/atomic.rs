use crate::{
    typed::Type,
    variable::{LockedVariable, Stage, Status},
    TypedVariable,
};
use async_atomic::Atomic as AsyncAtomic;
use futures::task::{waker_ref, ArcWake};
use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub struct AtomicVariable<T: Type> {
    variable: TypedVariable<T>,
    value: AsyncAtomic<T>,
    update: AtomicBool,
}

impl<T: Type + Default> AtomicVariable<T> {
    pub fn new(variable: TypedVariable<T>) -> Arc<Self> {
        let this = Arc::new(Self {
            variable,
            value: AsyncAtomic::default(),
            update: AtomicBool::new(false),
        });
        this.notify(&mut this.variable.lock());
        this
    }
}

impl<T: Type> AtomicVariable<T> {
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
                    *(locked.value_ptr() as *mut T) = self.value.load();
                } else {
                    self.value.store(*(locked.value_ptr() as *const T));
                }
                locked.commit(Status::Ok(()));
            },
            Stage::Commited => (),
        }
    }
    pub fn store(self: &Arc<Self>, value: T) {
        self.value.store(value);
        self.update.store(true, Ordering::Release);
        self.notify(&mut self.variable.lock());
    }
}

impl<T: Type> ArcWake for AtomicVariable<T> {
    fn wake_by_ref(this: &Arc<Self>) {
        // Variable is already locked when waker is called.
        let mut not_locked = unsafe { LockedVariable::without_lock(&this.variable) };
        this.notify(&mut not_locked);
    }
}

impl<T: Type> Deref for AtomicVariable<T> {
    type Target = AsyncAtomic<T>;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
