use std::{hash::Hash, ops::Deref};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use string_interner::{StringInterner, Symbol, backend::StringBackend};

use crate::id::raw::IdRaw;

pub type Interner = StringInterner<StringBackend<Id>>;

#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Id(u32);

impl Symbol for Id {
    fn try_from_usize(index: usize) -> Option<Self> {
        Some(Self(index as u32))
    }

    fn to_usize(self) -> usize {
        self.0 as usize
    }
}

impl Id {
    pub const NO_NAMEPSACE: Option<&'static str> = None;

    pub fn parse(
        id: &str,
        interner: &mut Interner,
        fallback_namespace: Option<impl AsRef<str>>,
    ) -> Option<Id> {
        IdRaw::parse(id, fallback_namespace).map(|id| id.to_id(interner))
    }

    pub fn try_parse(id: &str, interner: &Interner) -> Option<Id> {
        IdRaw::try_parse(id).and_then(|id| id.try_to_id(interner))
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct TileId(pub Id);
impl Deref for TileId {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ModelId(pub Id);
impl Deref for ModelId {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct RenderId(pub Id);
impl Deref for RenderId {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub mod raw {
    use core::{fmt::Display, ops::Deref};
    use std::sync::Arc;

    use super::{Id, Interner};

    #[derive(Debug, Clone)]
    pub struct IdRaw(Arc<str>);

    impl Display for IdRaw {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl Deref for IdRaw {
        type Target = Arc<str>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl AsRef<str> for IdRaw {
        fn as_ref(&self) -> &str {
            self
        }
    }

    impl IdRaw {
        pub fn to_id(&self, interner: &mut Interner) -> Id {
            interner.get_or_intern(self)
        }

        pub fn try_to_id(&self, interner: &Interner) -> Option<Id> {
            interner.get(self)
        }

        pub fn new(a: &str, b: &str) -> Self {
            assert!(!a.is_empty());
            assert!(!b.is_empty());

            IdRaw(Arc::from(format!("{a}:{b}")))
        }

        pub fn try_parse(s: &str) -> Option<Self> {
            s.split_once(':').map(|(a, b)| Self::new(a, b))
        }

        pub fn parse(s: &str, fallback_namespace: Option<impl AsRef<str>>) -> Option<Self> {
            Self::try_parse(s).or_else(|| fallback_namespace.map(|v| Self::new(v.as_ref(), s)))
        }
    }
}
