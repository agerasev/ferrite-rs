use atomic_enum::atomic_enum;
use futures::task::AtomicWaker;
use std::{
    ffi::CStr,
    ops::{Deref, DerefMut},
    os::raw::c_void,
    sync::atomic::Ordering,
    task::Waker,
};

use super::import::*;
pub use super::import::{
    FerVarAction as Action, FerVarInfo as Info, FerVarPerm as Perm, FerVarType as Type,
    FerVarValue as Value,
};

#[repr(transparent)]
pub struct VariableBase {
    raw: *mut FerVar,
}

unsafe impl Send for VariableBase {}

impl VariableBase {
    pub unsafe fn from_raw(raw: *mut FerVar) -> Self {
        Self { raw }
    }

    pub fn name(&self) -> &CStr {
        unsafe { CStr::from_ptr(fer_var_name(self.raw)) }
    }
    pub fn info(&self) -> Info {
        unsafe { fer_var_info(self.raw) }
    }
    pub fn value(&self) -> &Value {
        unsafe { &*fer_var_value(self.raw) }
    }
    pub fn value_mut(&mut self) -> &mut Value {
        unsafe { &mut *fer_var_value(self.raw) }
    }

    pub fn state(&self) -> &State {
        unsafe { (self.user_data() as *const State).as_ref() }.unwrap()
    }
    fn user_data(&self) -> *mut c_void {
        unsafe { fer_var_user_data(self.raw) }
    }
    fn set_user_data(&mut self, user_data: *mut c_void) {
        unsafe { fer_var_set_user_data(self.raw, user_data) }
    }
}

#[repr(transparent)]
pub struct VariableUnprotected {
    base: VariableBase,
}

impl VariableUnprotected {
    pub unsafe fn from_raw(raw: *mut FerVar) -> Self {
        Self {
            base: VariableBase::from_raw(raw),
        }
    }

    pub fn init(&mut self) {
        assert!(self.user_data().is_null());
        let info = Box::new(State::new());
        self.set_user_data(Box::into_raw(info) as *mut c_void);
    }

    pub unsafe fn request_proc(&mut self) {
        let prev = self.state().swap_proc_state(ProcState::Requested);
        debug_assert_eq!(prev, ProcState::Idle);
        fer_var_request(self.raw);
    }
    pub unsafe fn proc_begin(&mut self) {
        let state = self.state();
        let prev = state.swap_proc_state(ProcState::Processing);
        debug_assert!(prev == ProcState::Idle || prev == ProcState::Requested);
        state.try_wake();
    }
    pub unsafe fn commit(&mut self, action: Action) {
        let prev = self.state().swap_proc_state(ProcState::Commited);
        debug_assert_eq!(prev, ProcState::Processing);
        fer_var_commit(self.raw, action);
    }
    pub unsafe fn proc_end(&mut self) {
        let state = self.state();
        let prev = state.swap_proc_state(ProcState::Idle);
        debug_assert_eq!(prev, ProcState::Commited);
        state.try_wake();
    }

    pub unsafe fn lock(&self) {
        fer_var_lock(self.raw);
    }
    pub unsafe fn unlock(&self) {
        fer_var_unlock(self.raw);
    }
}

impl Deref for VariableUnprotected {
    type Target = VariableBase;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl DerefMut for VariableUnprotected {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[repr(transparent)]
pub struct Variable {
    var: VariableUnprotected,
}

impl Variable {
    pub unsafe fn new(var: VariableUnprotected) -> Self {
        Self { var }
    }
    #[allow(dead_code)]
    pub unsafe fn into_inner(self) -> VariableUnprotected {
        self.var
    }

    pub unsafe fn get_unprotected(&self) -> &VariableUnprotected {
        &self.var
    }
    pub unsafe fn get_unprotected_mut(&mut self) -> &mut VariableUnprotected {
        &mut self.var
    }

    pub fn lock(&mut self) -> Guard<'_> {
        Guard::new(&mut self.var)
    }
}

impl Deref for Variable {
    type Target = VariableBase;
    fn deref(&self) -> &Self::Target {
        &self.var
    }
}
impl DerefMut for Variable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.var
    }
}

pub struct Guard<'a> {
    var: &'a mut VariableUnprotected,
}
impl<'a> Guard<'a> {
    fn new(var: &'a mut VariableUnprotected) -> Self {
        unsafe { var.lock() };
        Self { var }
    }
}
impl<'a> Deref for Guard<'a> {
    type Target = VariableUnprotected;
    fn deref(&self) -> &VariableUnprotected {
        self.var
    }
}
impl<'a> DerefMut for Guard<'a> {
    fn deref_mut(&mut self) -> &mut VariableUnprotected {
        self.var
    }
}
impl<'a> Drop for Guard<'a> {
    fn drop(&mut self) {
        unsafe { self.var.unlock() };
    }
}

#[atomic_enum]
#[derive(Eq, PartialEq)]
pub enum ProcState {
    Idle = 0,
    Requested,
    Processing,
    Commited,
}

pub struct State {
    proc_state: AtomicProcState,
    waker: AtomicWaker,
}

impl State {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            proc_state: AtomicProcState::new(ProcState::Idle),
            waker: AtomicWaker::new(),
        }
    }

    pub fn proc_state(&self) -> ProcState {
        self.proc_state.load(Ordering::Acquire)
    }
    fn swap_proc_state(&self, prev: ProcState) -> ProcState {
        self.proc_state.swap(prev, Ordering::SeqCst)
    }

    pub fn set_waker(&self, waker: &Waker) {
        self.waker.register(waker);
    }
    fn try_wake(&self) {
        self.waker.wake();
    }
}

unsafe impl Send for State {}
