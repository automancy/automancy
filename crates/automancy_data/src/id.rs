use core::{
    cell::UnsafeCell,
    fmt::{Debug, Display},
};
use std::{hash::Hash, ops::Deref};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
pub use string_interner::Symbol as InternSymbol;

use crate::id::deserialize::StrIdRef;

// TODO investigate perf
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct IdInterner(string_interner::StringInterner<string_interner::backend::BucketBackend<Id>>);

thread_local! {
    pub static GLOBAL_INTERNER: UnsafeCell<IdInterner> = UnsafeCell::new(IdInterner::new());
}

impl Default for IdInterner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl IdInterner {
    pub fn new() -> Self {
        let mut interner = string_interner::StringInterner::with_capacity(128);

        assert_eq!(
            interner.get_or_intern_static(deserialize::NONE),
            Id::none(),
            "Id::none needs to equal 0"
        );
        assert_eq!(
            interner.get_or_intern_static(deserialize::ANY),
            Id::any(),
            "Id::any needs to equal 1"
        );
        assert_eq!(
            interner.get_or_intern_static(deserialize::ANY),
            Id::none(),
            "Id::any needs to equal Id::none"
        );

        Self(interner)
    }

    /// Returns the string for the given `symbol`` if any.
    #[inline]
    pub fn resolve(&self, symbol: Id) -> Option<&str> {
        self.0.resolve(symbol)
    }

    /// Returns the symbol for the given string if any.
    ///
    /// Can be used to query if a string has already been interned without interning.
    #[inline]
    pub fn get<'a>(&self, string: StrIdRef<'a>) -> Option<Id> {
        self.0.get(string)
    }

    fn parse_string_id<'a>(
        string: &'a str,
        fallback_namespace: Option<&'a str>,
    ) -> Result<(&'a str, &'a str), deserialize::StrIdParseError> {
        if let Some((namespace, name)) = string.split_once(deserialize::DELIM) {
            if name.contains(deserialize::DELIM) {
                Err(deserialize::StrIdParseError::ExtraDelims(string.to_owned()))
            } else {
                Ok((namespace, name))
            }
        } else if let Some(fallback_namespace) = fallback_namespace {
            Ok((fallback_namespace, string))
        } else {
            Err(deserialize::StrIdParseError::NoDelimNoFallback(string.to_owned()))
        }
    }

    /// Interns the given string.
    ///
    /// Returns a symbol for resolution into the original string.
    ///
    /// # Panics
    ///
    /// If the interner already interns the maximum number of strings possible
    /// by the chosen symbol type.
    #[inline]
    pub fn get_or_intern<'a>(
        &mut self,
        string: StrIdRef<'a>,
        fallback_namespace: Option<&str>,
    ) -> Result<Id, deserialize::StrIdParseError> {
        let (namespace, name) = Self::parse_string_id(&string, fallback_namespace)?;

        match name {
            deserialize::NONE => Ok(Id::none()),
            deserialize::ANY => Ok(Id::any()),
            _ => Ok(self.0.get_or_intern(format!("{namespace}{}{name}", deserialize::DELIM))),
        }
    }

    #[inline]
    pub fn get_or_intern_opt<'a>(
        &mut self,
        string: Option<StrIdRef<'a>>,
        fallback_namespace: Option<&str>,
    ) -> Result<Id, deserialize::StrIdParseError> {
        if let Some(string) = string {
            self.get_or_intern(string, fallback_namespace)
        } else {
            Ok(Id::none())
        }
    }

    /// Interns the given `'static` string.
    ///
    /// Returns a symbol for resolution into the original string.
    ///
    /// # Note
    ///
    /// This is more efficient than [`Self::get_or_intern`] since it might
    /// avoid some memory allocations if the backends supports this.
    ///
    /// # Panics
    ///
    /// If the interner already interns the maximum number of strings possible
    /// by the chosen symbol type.
    #[inline]
    pub fn get_or_intern_static(&mut self, string: &'static str) -> Id {
        let (_namespace, name) = Self::parse_string_id(string, None).unwrap();

        match name {
            deserialize::NONE => Id::none(),
            deserialize::ANY => Id::any(),
            _ => self.0.get_or_intern_static(string),
        }
    }
}

/**
Implements a newtype that wraps around [`Id`].

Example:
```
use automancy_data::id::{Id, TileId};

let tile_id = TileId(Id::from(0));
assert_eq!("TileId(none)", tile_id.to_string());
assert_eq!(*tile_id, Id::from(0));
```
*/
macro_rules! impl_id_newtype {
    ($ty:ident) => {
        #[must_use]
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Pod, Zeroable)]
        pub struct $ty(pub Id);

        impl Deref for $ty {
            type Target = Id;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<Id> for $ty {
            fn from(value: Id) -> Self {
                Self(value)
            }
        }

        impl From<$ty> for Id {
            fn from(value: $ty) -> Self {
                *value
            }
        }

        impl Debug for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!(concat!(stringify!($ty), "({})"), self.0.0))
            }
        }

        impl Display for $ty {
            #[cfg_attr(feature = "profile", profiling::function)]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($ty), "("))?;
                GLOBAL_INTERNER.with(|interner| {
                    let interner = unsafe { &*interner.get() };
                    if let Some(s) = interner.resolve(**self) {
                        f.write_str(s)
                    } else {
                        f.write_str("invalid")
                    }
                })?;
                f.write_str(")")?;
                Ok(())
            }
        }

        impl $ty {
            /// Represents an invalid Id.
            pub const fn invalid() -> Self {
                Self(Id::invalid())
            }

            /// Static Id named [`none`](deserialize::StrId::NONE), with no namespace.
            ///
            /// Any Ids with the same *name* will be converted to this, regardless of namespace.
            pub const fn none() -> Self {
                Self(Id::none())
            }

            /// Static Id named [`any`](deserialize::StrId::ANY), with no namespace.
            ///
            /// Any Ids with the same *name* will be converted to this, regardless of namespace.
            pub const fn any() -> Self {
                Self(Id::any())
            }
        }

        impl IdLike for $ty {}
    };

    ($ty: ident, $($tys: ident),+) => {
        impl_id_newtype!($ty);
        impl_id_newtype!($($tys),*);
    }
}

#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialOrd, Ord, Serialize, Deserialize, Pod, Zeroable)]
#[serde(transparent)]
pub struct Id(u32);

pub trait IdLike: Into<Id> + From<Id> + Copy + Eq + Ord + Hash {}
impl IdLike for Id {}

impl Hash for Id {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u32(self.0);
    }
}

impl nohash_hasher::IsEnabled for Id {}

impl PartialEq for Id {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || (self.0 == Id::any().0 || other.0 == Id::any().0)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::none()
    }
}

impl Id {
    /// Represents an invalid Id.
    pub const fn invalid() -> Self {
        Self(u32::MAX)
    }

    /// Static Id named [`none`](deserialize::StrId::NONE), with no namespace.
    ///
    /// Any Ids with the same *name* will be converted to this, regardless of namespace.   #[inline]
    pub const fn none() -> Self {
        Self(0)
    }

    /// Static Id named [`any`](deserialize::StrId::ANY), with no namespace.
    ///
    /// Any Ids with the same *name* will be converted to this, regardless of namespace.  #[inline]
    pub const fn any() -> Self {
        Self(1)
    }

    pub fn is_invalid(&self) -> bool {
        self.0 == Self::invalid().0
    }

    pub fn is_none(&self) -> bool {
        self.0 == Self::none().0
    }

    pub fn is_any(&self) -> bool {
        self.0 == Self::any().0
    }

    pub fn is_built_in(&self) -> bool {
        self.is_none() || self.is_any()
    }
}

impl From<u32> for Id {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Id({})", self.0))
    }
}

impl Display for Id {
    #[cfg_attr(feature = "profile", profiling::function)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Id(")?;
        GLOBAL_INTERNER.with(|interner| {
            let interner = unsafe { &*interner.get() };
            if let Some(s) = interner.resolve(*self) {
                f.write_str(s)
            } else {
                f.write_str("invalid")
            }
        })?;
        f.write_str(")")?;
        Ok(())
    }
}

impl InternSymbol for Id {
    fn try_from_usize(index: usize) -> Option<Self> {
        Some(Self(index as u32))
    }

    fn to_usize(self) -> usize {
        self.0 as usize
    }
}

impl_id_newtype!(
    TileId, ItemId, RecipeId, TagId, CategoryId, ResearchId, ScriptId, ModelId, RenderId, UiRenderId
);

pub mod deserialize {
    use std::borrow::Cow;

    use thiserror::Error;

    pub const DELIM: &str = ":";

    pub const NONE: &str = "none";
    pub const ANY: &str = "any";

    pub type StrIdRef<'a> = Cow<'a, str>;
    pub type StrId = StrIdRef<'static>;

    #[derive(Debug, Error)]
    pub enum StrIdParseError {
        #[error("more than 1 delimiter ':' found in {0}, there can only be 1 delimiter; Id should be in the format of 'namespace:name'")]
        ExtraDelims(String),
        #[error("no delimiter ':' found in {0}, and there wasn't any sensible fallback namespace")]
        NoDelimNoFallback(String),
    }
}

pub mod parse {
    use crate::{
        game::inventory::{ItemStack, deserialize::ItemStackStr},
        id::{
            IdInterner, ItemId,
            deserialize::{StrIdParseError, StrIdRef},
        },
    };

    #[cfg_attr(feature = "profile", profiling::function)]
    pub fn parse_ids<'a, Id: From<crate::id::Id>>(
        iter: impl Iterator<Item = StrIdRef<'a>>,
        interner: &mut IdInterner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<Id, StrIdParseError>> {
        iter.map(move |id| interner.get_or_intern(id, namespace).map(From::from))
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    pub fn parse_map_id_item<'a, Id: From<crate::id::Id>, Item, ResultItem: From<Item>>(
        iter: impl Iterator<Item = (StrIdRef<'a>, Item)>,
        interner: &mut IdInterner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<(Id, ResultItem), StrIdParseError>> {
        iter.map(move |(id, item)| interner.get_or_intern(id, namespace).map(|id| (Id::from(id), item.into())))
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    pub fn parse_map_id_static_str<'a, Id: From<crate::id::Id>, Item: Into<String>>(
        iter: impl Iterator<Item = (StrIdRef<'a>, Item)>,
        interner: &mut IdInterner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<(Id, &'static str), StrIdParseError>> {
        iter.map(move |(id, item)| interner.get_or_intern(id, namespace).map(|id| (Id::from(id), &*item.into().leak())))
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    pub fn parse_map_item_id<'a, Id: From<crate::id::Id>, Item, ResultItem: From<Item>>(
        iter: impl Iterator<Item = (Item, StrIdRef<'a>)>,
        interner: &mut IdInterner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<(ResultItem, Id), StrIdParseError>> {
        iter.map(move |(item, id)| interner.get_or_intern(id, namespace).map(|id| (item.into(), Id::from(id))))
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    pub fn parse_item_stacks(
        iter: impl Iterator<Item = ItemStackStr>,
        interner: &mut IdInterner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<ItemStack, StrIdParseError>> {
        iter.map(move |stack| {
            interner.get_or_intern(stack.id, namespace).map(|id| ItemStack {
                id: ItemId(id),
                amount: stack.amount,
            })
        })
    }
}
