use atomic::Atomic;
use derive_more::{Deref, DerefMut};
use futures::task::AtomicWaker;
use std::{
    ffi::CStr,
    mem::ManuallyDrop,
    os::raw::{c_char, c_void},
    ptr,
    str::from_utf8,
    sync::atomic::Ordering,
    task::Waker,
};

use super::import::*;
pub use super::import::{FerVarInfo as Info, FerVarType as Type, FerVarValue as Value};

pub type Status<'a> = Result<(), &'a str>;

/// Basic variable.
///
/// Allowed to have multiple instances of the same variable.
#[repr(transparent)]
pub struct Variable {
    raw: *mut FerVar,
}

unsafe impl Send for Variable {}
unsafe impl Sync for Variable {}

impl Variable {
    pub(crate) unsafe fn from_raw(raw: *mut FerVar) -> Self {
        Self { raw }
    }

    pub fn name(&self) -> &str {
        from_utf8(unsafe { CStr::from_ptr(fer_var_name(self.raw)) }.to_bytes()).unwrap()
    }
    pub fn info(&self) -> Info {
        unsafe { fer_var_info(self.raw) }
    }

    pub(crate) fn value_ptr(&self) -> *mut Value {
        unsafe { fer_var_value(self.raw) }
    }

    fn user_data(&self) -> *mut c_void {
        unsafe { fer_var_user_data(self.raw) }
    }
    pub(crate) fn state(&self) -> &SharedState {
        unsafe { (self.user_data() as *const SharedState).as_ref() }.unwrap()
    }

    pub(crate) fn lock(&self) -> LockedVariable<'_> {
        unsafe {
            fer_var_lock(self.raw);
            LockedVariable { base: self }
        }
    }
}

/// System-side mutable variable part.
///
/// Mutually exclusive with [`LockedVariable`].
#[repr(transparent)]
#[derive(Deref, DerefMut)]
pub(crate) struct SystemVariable {
    base: Variable,
}

impl SystemVariable {
    pub unsafe fn from_raw(raw: *mut FerVar) -> Self {
        Self {
            base: Variable::from_raw(raw),
        }
    }
    pub unsafe fn initialize(&mut self) {
        assert!(self.user_data().is_null());
        let info = Box::new(SharedState::new());
        self.set_user_data(Box::into_raw(info) as *mut c_void);
    }
    unsafe fn set_user_data(&mut self, user_data: *mut c_void) {
        unsafe { fer_var_set_user_data(self.raw, user_data) }
    }

    pub unsafe fn proc_begin(&mut self) {
        let state = self.state();
        let prev = state.swap_stage(Stage::Processing);
        debug_assert!(prev == Stage::Idle || prev == Stage::Requested);
        state.waker.wake();
    }
    pub unsafe fn proc_end(&mut self) {
        let state = self.state();
        let prev = state.swap_stage(Stage::Idle);
        debug_assert_eq!(prev, Stage::Commited);
        state.waker.wake();
    }
}

/// User-side mutable variable part.
///
/// Mutually exclusive with [`SystemVariable`] and other instances of [`LockedVariable`].
#[repr(transparent)]
#[derive(Deref, DerefMut)]
pub(crate) struct LockedVariable<'a> {
    base: &'a Variable,
}

impl<'a> LockedVariable<'a> {
    pub unsafe fn without_lock(base: &'a Variable) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Self { base })
    }
    pub unsafe fn request_proc(&mut self) {
        let prev = self.state().swap_stage(Stage::Requested);
        debug_assert_eq!(prev, Stage::Idle);
        fer_var_request(self.raw);
    }
    pub unsafe fn commit(&mut self, status: Status<'_>) {
        let prev = self.state().swap_stage(Stage::Commited);
        debug_assert_eq!(prev, Stage::Processing);

        match status {
            Ok(()) => fer_var_commit(self.raw, FerVarStatus::Ok, ptr::null(), 0),
            Err(message) => fer_var_commit(
                self.raw,
                FerVarStatus::Error,
                message.as_ptr() as *const c_char,
                message.as_bytes().len(),
            ),
        };
    }
}

impl<'a> Drop for LockedVariable<'a> {
    fn drop(&mut self) {
        unsafe { fer_var_unlock(self.raw) };
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub(crate) enum Stage {
    Idle = 0,
    Requested,
    Processing,
    Commited,
}

pub(crate) struct SharedState {
    stage: Atomic<Stage>,
    waker: AtomicWaker,
}

impl SharedState {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            stage: Atomic::new(Stage::Idle),
            waker: AtomicWaker::new(),
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage.load(Ordering::Acquire)
    }
    fn swap_stage(&self, prev: Stage) -> Stage {
        self.stage.swap(prev, Ordering::SeqCst)
    }

    pub fn set_waker(&self, waker: &Waker) {
        self.waker.register(waker);
    }
}

unsafe impl Send for SharedState {}
unsafe impl Sync for SharedState {}
