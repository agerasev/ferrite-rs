use crate::{typed::Value, TypedVariable, Variable};
use std::any::TypeId;

pub trait Downcast<V> {
    fn downcast(self) -> Option<V>;
}

impl<T: Value> Downcast<TypedVariable<T>> for Variable {
    fn downcast(self) -> Option<TypedVariable<T>> {
        if self.info().type_.type_id() == TypeId::of::<T>() && self.info().max_len == 0 {
            Some(unsafe { TypedVariable::new_unchecked(self) })
        } else {
            None
        }
    }
}
impl<T: Value> Downcast<TypedVariable<[T]>> for Variable {
    fn downcast(self) -> Option<TypedVariable<[T]>> {
        if self.info().type_.type_id() == TypeId::of::<T>() {
            Some(unsafe { TypedVariable::new_unchecked(self) })
        } else {
            None
        }
    }
}
