use core::fmt::Debug;
use std::any::type_name;

#[repr(transparent)]
#[derive(
    Clone,
    Copy,
    PartialOrd,
    Ord,
    Default,
    Hash,
    derive_more::PartialEq,
    derive_more::Eq,
    derive_more::AsRef,
    derive_more::AsMut,
    derive_more::Deref,
    derive_more::DerefMut,
)]
pub struct IgnoreDebug<T>(pub T);

impl<T> Debug for IgnoreDebug<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "opaque {}", type_name::<T>())
    }
}
