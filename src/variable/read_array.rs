use std::{
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use super::WriteArrayVariable;
use crate::raw::{self, variable::ProcState};

#[repr(transparent)]
pub struct ReadArrayVariable<T: Copy> {
    raw: raw::Variable,
    _phantom: PhantomData<T>,
}

impl<T: Copy> ReadArrayVariable<T> {
    pub(crate) fn from_raw(raw: raw::Variable) -> Self {
        Self {
            raw,
            _phantom: PhantomData,
        }
    }

    pub fn read_in_place(&mut self) -> ReadInPlaceFuture<'_, T> {
        ReadInPlaceFuture { owner: Some(self) }
    }

    pub async fn read_to_slice(&mut self, dst: &mut [T]) -> Option<usize> {
        let src = self.read_in_place().await;
        let res = if dst.len() >= src.len() {
            dst[..src.len()].copy_from_slice(&src);
            Some(src.len())
        } else {
            None
        };
        src.close().await;
        res
    }
}

impl<T: Copy> Deref for ReadArrayVariable<T> {
    type Target = WriteArrayVariable<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const _ as *const WriteArrayVariable<T>) }
    }
}
impl<T: Copy> DerefMut for ReadArrayVariable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self as *mut _ as *mut WriteArrayVariable<T>) }
    }
}
