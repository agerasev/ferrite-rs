use crate::{Downcast, Info, Variable};
use derive_more::{Deref, DerefMut, Display, Error};
use lazy_static::lazy_static;
use std::{collections::HashMap, mem, sync::Mutex};

#[derive(Deref, DerefMut)]
#[repr(transparent)]
pub struct Registry(HashMap<String, Variable>);

lazy_static! {
    static ref REGISTRY: Mutex<Registry> = Mutex::new(Registry(HashMap::new()));
}

pub(crate) fn add_variable(var: Variable) {
    assert!(REGISTRY
        .lock()
        .unwrap()
        .insert(var.name().into(), var)
        .is_none());
}

pub(crate) fn take() -> Registry {
    let mut ret = Registry(HashMap::new());
    mem::swap(&mut *REGISTRY.lock().unwrap(), &mut ret);
    ret
}

#[derive(Clone, Debug, Display)]
pub enum GetDowncastErrorKind {
    #[display(fmt = "Not found")]
    NotFound,
    #[display(fmt = "Wrong type, {:?} expected", "_0")]
    WrongType(Info),
}

#[derive(Clone, Debug, Display, Error)]
#[display(fmt = "PV '{}': {}", "name", "kind")]
pub struct GetDowncastError {
    name: String,
    kind: GetDowncastErrorKind,
}

#[derive(Clone, Debug, Display, Error)]
#[display(fmt = "There are unused PVs: {:?}", "_0")]
pub struct CheckEmptyError(#[error(not(source))] pub Vec<String>);

impl Registry {
    pub fn remove_downcast<V>(&mut self, name: &str) -> Result<V, GetDowncastError>
    where
        Variable: Downcast<V>,
    {
        log::debug!("take: {}", name);
        let var = match self.remove(name) {
            Some(var) => var,
            None => {
                return Err(GetDowncastError {
                    name: name.into(),
                    kind: GetDowncastErrorKind::NotFound,
                })
            }
        };
        let info = var.info();
        match var.downcast() {
            Some(var) => Ok(var),
            None => Err(GetDowncastError {
                name: name.into(),
                kind: GetDowncastErrorKind::WrongType(info),
            }),
        }
    }

    pub fn remove_downcast_suffix<V>(&mut self, suffix: &str) -> Result<V, GetDowncastError>
    where
        Variable: Downcast<V>,
    {
        match self
            .keys()
            .find(|name| is_suffix(name, suffix))
            .map(|s| s.clone())
        {
            Some(name) => self.remove_downcast(&name),
            None => Err(GetDowncastError {
                name: format!("*{}", suffix),
                kind: GetDowncastErrorKind::NotFound,
            }),
        }
    }

    pub fn check_empty(&self) -> Result<(), CheckEmptyError> {
        if !self.is_empty() {
            Err(CheckEmptyError(self.keys().map(String::from).collect()))
        } else {
            Ok(())
        }
    }
}

fn is_suffix(text: &str, suffix: &str) -> bool {
    if text.ends_with(suffix) {
        if text.len() == suffix.len() {
            return true;
        }
        let c = text.chars().nth(text.len() - suffix.len() - 1).unwrap();
        if c.is_ascii_punctuation() && c != '_' {
            return true;
        }
    }
    false
}
