use std::any::TypeId;

use super::{ArrayVariable, Variable};
use crate::raw::{
    self,
    variable::{Info, Perm},
};

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
    pub unsafe fn new(raw: raw::Variable) -> Self {
        Self { raw }
    }

    pub fn downcast_scalar<T: Copy + 'static, const R: bool, const W: bool, const A: bool>(
        self,
    ) -> Option<Variable<T, R, W, A>> {
        let perm = self.info().perm;
        if (!R || perm.contains(Perm::READ))
            && (!W || perm.contains(Perm::WRITE))
            && (!A || perm.contains(Perm::REQUEST))
        {
            if self.info().type_.type_id() == TypeId::of::<T>() {
                Some(unsafe { Variable::from_any(self) })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn downcast_array<T: Copy + 'static, const R: bool, const W: bool, const A: bool>(
        self,
    ) -> Option<ArrayVariable<T, R, W, A>> {
        let perm = self.info().perm;
        if (!R || perm.contains(Perm::READ))
            && (!W || perm.contains(Perm::WRITE))
            && (!A || perm.contains(Perm::REQUEST))
        {
            if self.info().type_.type_id() == TypeId::of::<T>() {
                Some(unsafe { ArrayVariable::from_any(self) })
            } else {
                None
            }
        } else {
            None
        }
    }
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
impl<T: Copy + 'static, const R: bool, const W: bool, const A: bool>
    Downcast<ArrayVariable<T, R, W, A>> for AnyVariable
{
    fn downcast(self) -> Option<ArrayVariable<T, R, W, A>> {
        self.downcast_array::<T, R, W, A>()
    }
}
