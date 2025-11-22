use core::fmt::Display;
use std::{hash::Hash, ops::Deref};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use string_interner::{StringInterner, Symbol, backend::StringBackend};

pub type Interner = StringInterner<StringBackend<Id>>;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(u32);

impl From<u32> for Id {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

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

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct TileId(pub Id);
impl Deref for TileId {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for TileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("TileId({})", self.0.0))
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

impl Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("ModelId({})", self.0.0))
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

impl Display for RenderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("RenderId({})", self.0.0))
    }
}

pub mod deserialize {
    use core::{fmt::Display, ops::Deref};

    use serde::Deserialize;
    use thiserror::Error;

    use super::{Id, Interner};

    #[derive(Debug, Error)]
    pub enum StrIdParseError {
        #[error(
            "More than 1 delimiter ':' found in {0}. There can only be 1 delimiter! Ids should be in the format of 'namespace:name'. (If you want to separate the name anyway, use '/' instead.)"
        )]
        ExtraDelims(StrId),
        #[error("No delimiter ':' found in {0} and there wasn't any sensible fallback namespace.")]
        NoDelimNoFallback(StrId),
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
    #[serde(transparent)]
    pub struct StrId(String);

    impl Display for StrId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl Deref for StrId {
        type Target = str;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl AsRef<str> for StrId {
        fn as_ref(&self) -> &str {
            self
        }
    }

    impl From<String> for StrId {
        fn from(value: String) -> Self {
            Self(value)
        }
    }

    pub trait StrIdExt<T: Sized> {
        fn into_id(self, interner: &mut Interner, fallback_namespace: Option<&str>) -> Result<T, StrIdParseError>;
    }

    impl StrId {
        fn parse(self, fallback_namespace: Option<&str>) -> Result<StrId, StrIdParseError> {
            let delim_count = self.0.chars().filter(|v| *v == ':').count();

            if delim_count == 1 {
                Ok(self)
            } else if delim_count > 1 {
                Err(StrIdParseError::ExtraDelims(self))
            } else if let Some(fallback_namespace) = fallback_namespace {
                Ok(StrId::new(fallback_namespace, self.0.as_str()))
            } else {
                Err(StrIdParseError::NoDelimNoFallback(self))
            }
        }

        pub fn into_id(self, interner: &mut Interner, fallback_namespace: Option<&str>) -> Result<Id, StrIdParseError> {
            <Self as StrIdExt<Id>>::into_id(self, interner, fallback_namespace)
        }

        pub fn new(a: &str, b: &str) -> StrId {
            assert!(!a.is_empty());
            assert!(!b.is_empty());

            StrId(format!("{a}:{b}"))
        }
    }

    impl StrIdExt<Id> for StrId {
        fn into_id(self, interner: &mut Interner, fallback_namespace: Option<&str>) -> Result<Id, StrIdParseError> {
            Ok(interner.get_or_intern(self.parse(fallback_namespace)?.0))
        }
    }

    impl StrIdExt<Option<Id>> for Option<StrId> {
        fn into_id(self, interner: &mut Interner, fallback_namespace: Option<&str>) -> Result<Option<Id>, StrIdParseError> {
            match self {
                Some(v) => v.into_id(interner, fallback_namespace).map(Some),
                None => Ok(None),
            }
        }
    }
}

pub mod parse {
    use crate::{
        game::inventory::{ItemStack, deserialize::ItemStackStr},
        id::{
            Id, Interner,
            deserialize::{StrId, StrIdParseError},
        },
    };

    pub fn parse_ids(
        iter: impl Iterator<Item = StrId>,
        interner: &mut Interner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<Id, StrIdParseError>> {
        iter.map(move |id| id.into_id(interner, namespace))
    }

    pub fn parse_map_id_item<Item>(
        iter: impl Iterator<Item = (StrId, Item)>,
        interner: &mut Interner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<(Id, Item), StrIdParseError>> {
        iter.map(move |(id, item)| id.into_id(interner, namespace).map(|id| (id, item)))
    }

    pub fn parse_map_item_id<Item>(
        iter: impl Iterator<Item = (Item, StrId)>,
        interner: &mut Interner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<(Item, Id), StrIdParseError>> {
        iter.map(move |(item, id)| id.into_id(interner, namespace).map(|id| (item, id)))
    }

    pub fn parse_item_stacks(
        iter: impl Iterator<Item = ItemStackStr>,
        interner: &mut Interner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<ItemStack, StrIdParseError>> {
        iter.map(move |stack| stack.id.into_id(interner, namespace).map(|id| ItemStack { id, amount: stack.amount }))
    }
}
