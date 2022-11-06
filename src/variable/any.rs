use super::{
    typing::{Direction, VariableType},
    ReadArrayVariable, ReadVariable, WriteArrayVariable, WriteVariable,
};
use crate::raw;
use std::any::TypeId;

#[repr(transparent)]
pub struct AnyVariable {
    raw: raw::Variable,
}

impl AnyVariable {
    pub(crate) fn new(raw: raw::Variable) -> Self {
        Self { raw }
    }

    pub fn name(&self) -> String {
        self.raw.name().to_str().unwrap().to_owned()
    }

    fn raw_data_type(&self) -> raw::variable::Type {
        self.raw.data_type()
    }
    pub fn direction(&self) -> Direction {
        Direction::from_raw(self.raw_data_type().dir)
    }
    pub fn data_type(&self) -> VariableType {
        VariableType::from_raw(self.raw_data_type())
    }

    pub fn downcast_read<T: Copy + 'static + 'static>(self) -> Option<ReadVariable<T>> {
        match self.direction() {
            Direction::Read => match self.data_type() {
                VariableType::Scalar { scal_type } => {
                    if scal_type.type_id() == Some(TypeId::of::<T>()) {
                        Some(ReadVariable::from_raw(self.raw))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Direction::Write => None,
        }
    }
    pub fn downcast_write<T: Copy + 'static + 'static>(self) -> Option<WriteVariable<T>> {
        match self.direction() {
            Direction::Read => None,
            Direction::Write => match self.data_type() {
                VariableType::Scalar { scal_type } => {
                    if scal_type.type_id() == Some(TypeId::of::<T>()) {
                        Some(WriteVariable::from_raw(self.raw))
                    } else {
                        None
                    }
                }
                _ => None,
            },
        }
    }
    pub fn downcast_read_array<T: Copy + 'static + 'static>(self) -> Option<ReadArrayVariable<T>> {
        match self.direction() {
            Direction::Read => match self.data_type() {
                VariableType::Array { scal_type, .. } => {
                    if scal_type.type_id() == Some(TypeId::of::<T>()) {
                        Some(ReadArrayVariable::from_raw(self.raw))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Direction::Write => None,
        }
    }
    pub fn downcast_write_array<T: Copy + 'static + 'static>(
        self,
    ) -> Option<WriteArrayVariable<T>> {
        match self.direction() {
            Direction::Read => None,
            Direction::Write => match self.data_type() {
                VariableType::Array { scal_type, .. } => {
                    if scal_type.type_id() == Some(TypeId::of::<T>()) {
                        Some(WriteArrayVariable::from_raw(self.raw))
                    } else {
                        None
                    }
                }
                _ => None,
            },
        }
    }
}

pub trait Downcast<V> {
    fn downcast(self) -> Option<V>;
}

impl<T: Copy + 'static> Downcast<ReadVariable<T>> for AnyVariable {
    fn downcast(self) -> Option<ReadVariable<T>> {
        self.downcast_read::<T>()
    }
}

impl<T: Copy + 'static> Downcast<WriteVariable<T>> for AnyVariable {
    fn downcast(self) -> Option<WriteVariable<T>> {
        self.downcast_write::<T>()
    }
}

impl<T: Copy + 'static> Downcast<ReadArrayVariable<T>> for AnyVariable {
    fn downcast(self) -> Option<ReadArrayVariable<T>> {
        self.downcast_read_array::<T>()
    }
}

impl<T: Copy + 'static> Downcast<WriteArrayVariable<T>> for AnyVariable {
    fn downcast(self) -> Option<WriteArrayVariable<T>> {
        self.downcast_write_array::<T>()
    }
}
