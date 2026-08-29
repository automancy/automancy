use core::fmt::Debug;
use std::{
    any::type_name,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Default, Ord, Hash)]
#[repr(transparent)]
pub struct IgnoreDebug<T>(pub T);

impl<T> Debug for IgnoreDebug<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "opaque {}", type_name::<T>())
    }
}

impl<T> Deref for IgnoreDebug<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for IgnoreDebug<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
