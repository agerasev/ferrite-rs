use bitflags::bitflags;
use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
pub struct FerVar {
    _unused: [u8; 0],
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FerVarStatus {
    Ok = 0,
    Error,
}

bitflags! {
    pub struct FerVarPerm: u32 {
        const READ = 1;
        const WRITE = 2;
        const NOTIFY = 4;
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FerVarType {
    U8 = 0,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
    F64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FerVarInfo {
    pub perm: FerVarPerm,
    pub type_: FerVarType,
    pub max_len: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FerVarValue {
    pub data: *mut c_void,
    pub len: usize,
}

extern "C" {
    pub fn fer_app_exit(code: c_int);

    pub fn fer_var_request(var: *mut FerVar);
    pub fn fer_var_read_complete(var: *mut FerVar, status: FerVarStatus);
    pub fn fer_var_write_complete(var: *mut FerVar, status: FerVarStatus);

    pub fn fer_var_lock(var: *mut FerVar);
    pub fn fer_var_unlock(var: *mut FerVar);

    pub fn fer_var_name(var: *mut FerVar) -> *const c_char;
    pub fn fer_var_info(var: *mut FerVar) -> FerVarInfo;
    pub fn fer_var_value(var: *mut FerVar) -> *mut FerVarValue;

    pub fn fer_var_user_data(var: *mut FerVar) -> *mut c_void;
    pub fn fer_var_set_user_data(var: *mut FerVar, user_data: *mut c_void);
}
