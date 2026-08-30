use core::{
    cell::RefCell,
    fmt::{Debug, Display},
    num::NonZeroU32,
};
use std::{
    hash::Hash,
    ops::Deref,
    sync::{Arc, LazyLock, RwLock},
};

use bytemuck::{Pod, TransparentWrapper, Zeroable};
use serde::{Deserialize, Serialize};
pub use string_interner::Symbol as InternSymbol;

// TODO investigate perf
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct IdInterner(string_interner::StringInterner<string_interner::backend::BucketBackend<Id>>);

thread_local! {
    pub static MUTABLE_GLOBAL_INTERNER: RefCell<Option<IdInterner>> = const { RefCell::new(None) };
}

pub static GLOBAL_INTERNER: LazyLock<RwLock<Arc<IdInterner>>> = LazyLock::new(|| RwLock::new(Arc::new(IdInterner::new())));

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
    pub fn get(&self, string: &str) -> Option<Id> {
        self.0.get(string)
    }

    pub fn parse_string_id<'a>(
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
    pub fn get_or_intern(&mut self, string: &str, fallback_namespace: Option<&str>) -> Result<Id, deserialize::StrIdParseError> {
        let (namespace, name) = Self::parse_string_id(string, fallback_namespace)?;

        match name {
            deserialize::NONE => Ok(Id::none()),
            deserialize::ANY => Ok(Id::any()),
            _ => Ok(self.0.get_or_intern(format!("{namespace}{}{name}", deserialize::DELIM))),
        }
    }

    #[inline]
    pub fn get_or_intern_opt(
        &mut self,
        string: Option<&str>,
        fallback_namespace: Option<&str>,
    ) -> Result<Id, deserialize::StrIdParseError> {
        if let Some(string) = string {
            self.get_or_intern(string.as_ref(), fallback_namespace)
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
    ($( $ty:ident ),*) => {
        $(
            #[must_use]
            #[repr(transparent)]
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize, Pod, Zeroable)]
            pub struct $ty(pub Id);

            impl_id!($ty);

            impl IdNewType for $ty {}

            const impl Deref for $ty {
                type Target = Id;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            const impl From<Id> for $ty {
                fn from(value: Id) -> Self {
                    Self(value)
                }
            }

            const impl From<$ty> for Id {
                fn from(value: $ty) -> Self {
                    *value
                }
            }

            impl $ty {
                /// Returns the internal integer Id as an `u32`.
                #[inline]
                pub const fn into_inner(self) -> u32 {
                    self.0.into_inner()
                }

                /// Represents an invalid Id.
                #[inline]
                pub const fn invalid() -> Self {
                    Self(Id::invalid())
                }

                /// Static Id named [`none`](deserialize::NONE), with no namespace.
                ///
                /// Any Ids with the same *name* will be converted to this, regardless of namespace.
                #[inline]
                pub const fn none() -> Self {
                    Self(Id::none())
                }

                /// Static Id named [`any`](deserialize::ANY), with no namespace.
                ///
                /// Any Ids with the same *name* will be converted to this, regardless of namespace.
                #[inline]
                pub const fn any() -> Self {
                    Self(Id::any())
                }
            }
        )*

        paste::paste! {
            #[macro_export]
            macro_rules! for_each_id_new_type {
                {
                    NewType::$ident:ident$args:tt;
                } => {
                    $(
                        $ty::$ident$args;
                    )*
                };

                {
                    NewType::$ident:ident$args:tt?;
                } => {
                    $(
                        $ty::$ident$args?;
                    )*
                };

                {
                    $ident:ident!(NewType);
                } => {
                    $(
                        $ident!($ty);
                    )*
                };
            }
        }
    };
}

#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialOrd, Ord, Serialize, Deserialize, Pod, Zeroable)]
#[serde(transparent)]
pub struct Id(Option<NonZeroU32>);

/// Helper trait for blanket impl for [`Id`] + all [`Id`] new-types.
///
/// See also: [`impl_id_newtype`]
pub trait IdLike: Into<Id> + From<Id> + Copy + Eq + Ord + Hash + Display {}

/// Helper trait for blanket impl for all [`Id`] new-types.
/// Only use this if there's an impl conflict with using [`IdLike`].
///
/// See also: [`impl_id_newtype`]
pub trait IdNewType: IdLike {}

impl Hash for Id {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u32(self.into_inner());
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
    /// Returns the internal integer Id as an `u32`.
    #[inline]
    pub const fn into_inner(self) -> u32 {
        match self.0 {
            Some(v) => v.get(),
            None => 0,
        }
    }

    /// Represents an invalid Id.
    #[inline]
    pub const fn invalid() -> Self {
        Self(Some(NonZeroU32::MAX))
    }

    /// Static Id named [`none`](deserialize::NONE), with no namespace.
    ///
    /// Any Ids with the same *name* will be converted to this, regardless of namespace.
    #[inline]
    pub const fn none() -> Self {
        Self(None)
    }

    /// Static Id named [`any`](deserialize::ANY), with no namespace.
    ///
    /// Any Ids with the same *name* will be converted to this, regardless of namespace.
    #[inline]
    pub const fn any() -> Self {
        Self(NonZeroU32::new(1))
    }

    #[inline]
    pub fn is_invalid(&self) -> bool {
        self.0 == Self::invalid().0
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.0 == Self::none().0
    }

    #[inline]
    pub fn is_any(&self) -> bool {
        self.0 == Self::any().0
    }

    #[inline]
    pub fn is_built_in(&self) -> bool {
        self.is_none() || self.is_any()
    }
}

const impl From<u32> for Id {
    fn from(value: u32) -> Self {
        Self(NonZeroU32::new(value))
    }
}

impl InternSymbol for Id {
    fn try_from_usize(index: usize) -> Option<Self> {
        Some(Self::from(index as u32))
    }

    fn to_usize(self) -> usize {
        self.into_inner() as usize
    }
}

macro_rules! impl_id {
    ($ty:ident) => {
        unsafe impl TransparentWrapper<Id> for $ty {}

        impl IdLike for $ty {}

        impl Debug for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!(concat!(stringify!($ty), "({})"), self.into_inner()))
            }
        }

        impl Display for $ty {
            #[cfg_attr(feature = "profile", profiling::function)]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(stringify!($ty))?;
                f.write_str("(")?;

                // prefer the mutable interner
                MUTABLE_GLOBAL_INTERNER.with_borrow(|interner| match interner {
                    Some(interner) => {
                        if let Some(s) = interner.resolve(Id::from(*self)) {
                            f.write_str(s)
                        } else {
                            f.write_str("invalid")
                        }
                    },
                    None => {
                        let interner = GLOBAL_INTERNER.read().unwrap();
                        if let Some(s) = interner.resolve(Id::from(*self)) {
                            f.write_str(s)
                        } else {
                            f.write_str("invalid")
                        }
                    },
                })?;

                f.write_str(")")?;
                Ok(())
            }
        }
    };
}

impl_id!(Id);

impl_id_newtype!(
    TileId,
    ItemId,
    RecipeId,
    TagId,
    CategoryId,
    ResearchId,
    ScriptId,
    ModelId,
    RenderId,
    UiRenderId,
    GuiTranslateId,
    ResearchTranslateId
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
        iter.map(move |id| interner.get_or_intern(&id, namespace).map(From::from))
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    pub fn parse_map_id_item<'a, Id: From<crate::id::Id>, Item, ResultItem: From<Item>>(
        iter: impl Iterator<Item = (StrIdRef<'a>, Item)>,
        interner: &mut IdInterner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<(Id, ResultItem), StrIdParseError>> {
        iter.map(move |(id, item)| interner.get_or_intern(&id, namespace).map(|id| (Id::from(id), item.into())))
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    pub fn parse_map_id_static_str<'a, Id: From<crate::id::Id>, Item: Into<String>>(
        iter: impl Iterator<Item = (StrIdRef<'a>, Item)>,
        interner: &mut IdInterner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<(Id, &'static str), StrIdParseError>> {
        iter.map(move |(id, item)| {
            interner
                .get_or_intern(&id, namespace)
                .map(|id| (Id::from(id), &*item.into().leak()))
        })
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    pub fn parse_map_item_id<'a, Id: From<crate::id::Id>, Item, ResultItem: From<Item>>(
        iter: impl Iterator<Item = (Item, StrIdRef<'a>)>,
        interner: &mut IdInterner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<(ResultItem, Id), StrIdParseError>> {
        iter.map(move |(item, id)| interner.get_or_intern(&id, namespace).map(|id| (item.into(), Id::from(id))))
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    pub fn parse_item_stacks(
        iter: impl Iterator<Item = ItemStackStr>,
        interner: &mut IdInterner,
        namespace: Option<&str>,
    ) -> impl Iterator<Item = Result<ItemStack, StrIdParseError>> {
        iter.map(move |stack| {
            interner.get_or_intern(&stack.id, namespace).map(|id| ItemStack {
                id: ItemId(id),
                amount: stack.amount,
            })
        })
    }
}

#[allow(non_upper_case_globals)]
pub mod lua {
    use super::*;

    pub mod types {
        pub const Id: &str = "Id";
        pub const ItemId: &str = "ItemId";
        pub const TileId: &str = "ItemId";
    }

    macro_rules! impl_id {
        ($ty:ident) => {
            impl mlua::IntoLua for $ty {
                fn into_lua(self, _lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
                    Ok(mlua::Value::Integer(self.into_inner() as _))
                }
            }

            impl mlua::FromLua for $ty {
                fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
                    let v = value.as_integer().ok_or(mlua::Error::FromLuaConversionError {
                        from: value.type_name(),
                        to: stringify!($ty).to_string(),
                        message: None,
                    })?;

                    Ok(Self::from(Id::from(v as u32)))
                }
            }
        };
    }

    impl_id!(Id);

    for_each_id_new_type! {
        impl_id!(NewType);
    }
}
