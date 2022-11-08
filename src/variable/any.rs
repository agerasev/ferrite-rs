use super::Variable;
use crate::raw::{
    self,
    variable::{Info, Perm},
};
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

    pub fn info(&self) -> Info {
        self.raw.info()
    }
    pub fn downcast_scalar<T: Copy + 'static, const R: bool, const W: bool, const A: bool>(
        self,
    ) -> Option<Variable<T, R, W, A>> {
        let perm = self.info().perm;
        if (!R || perm.contains(Perm::READ))
            && (!W || perm.contains(Perm::WRITE))
            && (!A || perm.contains(Perm::NOTIFY))
        {
            if self.info().type_.type_id() == TypeId::of::<T>() {
                Some(Variable::from_raw(self.raw))
            } else {
                None
            }
        } else {
            None
        }
    }
    /*
    pub fn downcast_read_array<T: Copy + 'static>(self) -> Option<ReadArrayVariable<T>> {
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
    pub fn downcast_write_array<T: Copy + 'static>(self) -> Option<WriteArrayVariable<T>> {
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
    */
}

pub trait Downcast<V> {
    fn downcast(self) -> Option<V>;
}
impl<T: Copy + 'static, const R: bool, const W: bool, const A: bool> Downcast<Variable<T, R, W, A>>
    for AnyVariable
{
    fn downcast(self) -> Option<Variable<T, R, W, A>> {
        self.downcast_scalar::<T, R, W, A>()
    }
}
/*
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
*/
