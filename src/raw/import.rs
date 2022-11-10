use bitflags::bitflags;
use std::{
    any::TypeId,
    os::raw::{c_char, c_int, c_void},
};

#[repr(C)]
pub struct FerVar {
    _unused: [u8; 0],
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FerVarAction {
    Discard = 0,
    Read,
    Write,
}

bitflags! {
    #[repr(transparent)]
    pub struct FerVarPerm: u32 {
        const READ = 1;
        const WRITE = 2;
        const REQUEST = 4;
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

impl FerVarType {
    pub fn type_id(self) -> TypeId {
        match self {
            FerVarType::U8 => TypeId::of::<u8>(),
            FerVarType::I8 => TypeId::of::<i8>(),
            FerVarType::U16 => TypeId::of::<u16>(),
            FerVarType::I16 => TypeId::of::<i16>(),
            FerVarType::U32 => TypeId::of::<u32>(),
            FerVarType::I32 => TypeId::of::<i32>(),
            FerVarType::U64 => TypeId::of::<u64>(),
            FerVarType::I64 => TypeId::of::<i64>(),
            FerVarType::F32 => TypeId::of::<f32>(),
            FerVarType::F64 => TypeId::of::<f64>(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FerVarInfo {
    pub perm: FerVarPerm,
    pub type_: FerVarType,
    pub max_len: usize,
}

#[repr(C)]
pub struct FerVarValue {
    _unused: [u8; 0],
}

extern "C" {
    pub fn fer_app_exit(code: c_int);

    pub fn fer_var_request(var: *mut FerVar);
    pub fn fer_var_commit(var: *mut FerVar, action: FerVarAction);

    pub fn fer_var_lock(var: *mut FerVar);
    pub fn fer_var_unlock(var: *mut FerVar);

    pub fn fer_var_name(var: *mut FerVar) -> *const c_char;
    pub fn fer_var_info(var: *mut FerVar) -> FerVarInfo;
    pub fn fer_var_value(var: *mut FerVar) -> *mut FerVarValue;
    //pub fn fer_var_value_len(var: *mut FerVar) -> *mut usize;
    //pub fn fer_var_value_data(var: *mut FerVar) -> *mut c_void;

    pub fn fer_var_user_data(var: *mut FerVar) -> *mut c_void;
    pub fn fer_var_set_user_data(var: *mut FerVar, user_data: *mut c_void);
}
