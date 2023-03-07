#![allow(clippy::missing_safety_doc)]

use super::{import::*, variable::SystemVariable, Variable};
use crate::registry::{self, Registry};
use std::{
    panic::{self, PanicInfo},
    thread,
};

extern "Rust" {
    pub fn ferrite_app_main(variables: Registry);
}

#[no_mangle]
pub extern "C" fn fer_app_init() {
    let old_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info: &PanicInfo| {
        old_hook(info);
        unsafe { fer_app_exit(1) };
    }))
}

#[no_mangle]
pub extern "C" fn fer_app_start() {
    thread::spawn(move || unsafe {
        ferrite_app_main(registry::take());
        fer_app_exit(0);
    });
}

#[no_mangle]
pub unsafe extern "C" fn fer_var_init(ptr: *mut FerVar) {
    SystemVariable::from_raw(ptr).initialize();
    registry::add_variable(Variable::from_raw(ptr));
}

#[no_mangle]
pub unsafe extern "C" fn fer_var_proc_begin(ptr: *mut FerVar) {
    // No need for lock here - variable is already locked during this call.
    SystemVariable::from_raw(ptr).proc_begin();
}

#[no_mangle]
pub unsafe extern "C" fn fer_var_proc_end(ptr: *mut FerVar) {
    // No need for lock here - variable is already locked during this call.
    SystemVariable::from_raw(ptr).proc_end();
}
