use core::fmt::Display;
use std::{hash::Hash, ops::Deref};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use string_interner::{StringInterner, Symbol, backend::StringBackend};

pub type Interner = StringInterner<StringBackend<Id>>;

#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Id(u32);

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Id({})", self.0))
    }
}

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

pub mod deserialize {
    use core::{fmt::Display, ops::Deref};

    use serde::Deserialize;
    use thiserror::Error;

    use super::{Id, Interner};

    #[derive(Debug, Error)]
    pub enum IdStrParseError {
        #[error(
            "no delimiter ':' found in the id, ids need to be in the format of `namespace:name`"
        )]
        NoDelim,
        #[error("no delimiter ':' found in the id, and there wasn't any usable fallback namespace")]
        NoDelimNoFallback,
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
    #[serde(transparent)]
    pub struct IdStr(String);

    impl Display for IdStr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl Deref for IdStr {
        type Target = str;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl AsRef<str> for IdStr {
        fn as_ref(&self) -> &str {
            self
        }
    }

    impl IdStr {
        fn try_parse(s: &str) -> Result<IdStr, IdStrParseError> {
            s.split_once(':')
                .map(|(a, b)| Self::new(a, b))
                .ok_or(IdStrParseError::NoDelim)
        }

        fn parse(
            s: &str,
            fallback_namespace: Option<&impl AsRef<str>>,
        ) -> Result<IdStr, IdStrParseError> {
            Self::try_parse(s).or_else(|_| {
                fallback_namespace
                    .as_ref()
                    .map(|v| Self::new(v.as_ref(), s))
                    .ok_or(IdStrParseError::NoDelim)
            })
        }

        pub fn into_id(
            &self,
            interner: &mut Interner,
            fallback_namespace: Option<&impl AsRef<str>>,
        ) -> Result<Id, IdStrParseError> {
            Ok(interner.get_or_intern(Self::parse(&self.0, fallback_namespace)?))
        }

        pub fn new(a: &str, b: &str) -> Self {
            assert!(!a.is_empty());
            assert!(!b.is_empty());

            IdStr(format!("{a}:{b}"))
        }
    }
}

pub mod parse {
    use crate::{
        game::item::{ItemAmount, ItemStack},
        id::{
            Id, Interner,
            deserialize::{IdStr, IdStrParseError},
        },
    };

    pub fn parse_ids(
        iter: impl Iterator<Item = IdStr>,
        interner: &mut Interner,
        namespace: Option<&impl AsRef<str>>,
    ) -> impl Iterator<Item = Result<Id, IdStrParseError>> {
        iter.map(move |id| id.into_id(interner, namespace.as_ref()))
    }

    pub fn parse_map_id_item<Item>(
        iter: impl Iterator<Item = (IdStr, Item)>,
        interner: &mut Interner,
        namespace: Option<&impl AsRef<str>>,
    ) -> impl Iterator<Item = Result<(Id, Item), IdStrParseError>> {
        iter.map(move |(id, item)| {
            id.into_id(interner, namespace.as_ref())
                .map(|id| (id, item))
        })
    }

    pub fn parse_map_item_id<Item>(
        iter: impl Iterator<Item = (Item, IdStr)>,
        interner: &mut Interner,
        namespace: Option<&impl AsRef<str>>,
    ) -> impl Iterator<Item = Result<(Item, Id), IdStrParseError>> {
        iter.map(move |(item, id)| {
            id.into_id(interner, namespace.as_ref())
                .map(|id| (item, id))
        })
    }

    pub fn parse_item_stacks(
        iter: impl Iterator<Item = (IdStr, impl Into<ItemAmount>)>,
        interner: &mut Interner,
        namespace: Option<&impl AsRef<str>>,
    ) -> impl Iterator<Item = Result<ItemStack, IdStrParseError>> {
        iter.map(move |(id, amount)| {
            id.into_id(interner, namespace.as_ref())
                .map(|id| (id, amount.into()))
        })
        .map(|v| v.map(|(id, amount)| ItemStack { id, amount }))
    }
}
