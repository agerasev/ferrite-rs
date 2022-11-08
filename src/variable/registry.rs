use super::any::AnyVariable;
use lazy_static::lazy_static;
use std::{collections::HashMap, mem, sync::Mutex};

pub type Registry = HashMap<String, AnyVariable>;

lazy_static! {
    static ref REGISTRY: Mutex<Registry> = Mutex::new(HashMap::new());
}

pub(crate) fn add_variable(var: AnyVariable) {
    assert!(REGISTRY.lock().unwrap().insert(var.name(), var).is_none());
}

pub(crate) fn take() -> Registry {
    let mut ret = HashMap::new();
    mem::swap(&mut *REGISTRY.lock().unwrap(), &mut ret);
    ret
}
