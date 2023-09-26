use crate::{
    typed::Type,
    variable::{LockedVariable, Stage, Status},
    TypedVariable,
};
use async_atomic::Atomic as AsyncAtomic;
use futures::task::{waker_ref, ArcWake};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
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
            Stage::Committed => (),
        }
    }

    pub fn load(&self) -> T {
        self.value.load()
    }

    pub fn store(self: &Arc<Self>, value: T) {
        self.value.store(value);
        self.update.store(true, Ordering::Release);
        self.notify(&mut self.variable.lock());
    }
    pub fn swap(self: &Arc<Self>, value: T) -> T {
        let old = self.value.swap(value);
        self.update.store(true, Ordering::Release);
        self.notify(&mut self.variable.lock());
        old
    }
    pub fn compare_exchange(self: &Arc<Self>, current: T, new: T) -> Result<T, T> {
        let res = self.value.compare_exchange(current, new);
        if res.is_ok() {
            self.update.store(true, Ordering::Release);
            self.notify(&mut self.variable.lock());
        }
        res
    }
    pub fn fetch_update<F: FnMut(T) -> Option<T>>(self: &Arc<Self>, f: F) -> Result<T, T> {
        let res = self.value.fetch_update(f);
        if res.is_ok() {
            self.update.store(true, Ordering::Release);
            self.notify(&mut self.variable.lock());
        }
        res
    }
}

impl<T: Type> ArcWake for AtomicVariable<T> {
    fn wake_by_ref(this: &Arc<Self>) {
        // Variable is already locked when waker is called.
        let mut locked = unsafe { LockedVariable::without_lock(&this.variable) };
        this.notify(&mut locked);
    }
}

impl<T: Type> AsRef<AsyncAtomic<T>> for AtomicVariable<T> {
    fn as_ref(&self) -> &AsyncAtomic<T> {
        &self.value
    }
}
