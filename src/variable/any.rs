use std::any::TypeId;

use super::{ArrayVariable, Variable};
use crate::raw;

pub use raw::variable::Info;

pub trait Var: Sized {
    fn raw(&self) -> &raw::Variable;
    fn raw_mut(&mut self) -> &mut raw::Variable;

    fn name(&self) -> String {
        self.raw().name().to_str().unwrap().to_owned()
    }
    fn info(&self) -> Info {
        self.raw().info()
    }
}

#[repr(transparent)]
pub struct AnyVariable {
    raw: raw::Variable,
}

impl Var for AnyVariable {
    fn raw(&self) -> &raw::Variable {
        &self.raw
    }
    fn raw_mut(&mut self) -> &mut raw::Variable {
        &mut self.raw
    }
}

impl AnyVariable {
    /// # Safety
    ///
    /// There must be only one `AnyVariable` associated with raw variable at the moment.
    pub unsafe fn new(raw: raw::Variable) -> Self {
        Self { raw }
    }

    pub fn downcast_scalar<T: Copy + 'static>(self) -> Option<Variable<T>> {
        if self.info().type_.type_id() == TypeId::of::<T>() {
            Some(unsafe { Variable::from_any(self) })
        } else {
            None
        }
    }

    pub fn downcast_array<T: Copy + 'static>(self) -> Option<ArrayVariable<T>> {
        if self.info().type_.type_id() == TypeId::of::<T>() {
            Some(unsafe { ArrayVariable::from_any(self) })
        } else {
            None
        }
    }
}

pub trait Downcast<V> {
    fn downcast(self) -> Option<V>;
}

impl<T: Copy + 'static> Downcast<Variable<T>> for AnyVariable {
    fn downcast(self) -> Option<Variable<T>> {
        self.downcast_scalar::<T>()
    }
}
impl<T: Copy + 'static> Downcast<ArrayVariable<T>> for AnyVariable {
    fn downcast(self) -> Option<ArrayVariable<T>> {
        self.downcast_array::<T>()
    }
}
