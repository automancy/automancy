use std::fmt::{self, Display, Formatter};
use std::hash::Hash;

use bytemuck::{Pod, Zeroable};
use flexstr::SharedStr;
use rhai::INT;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use string_interner::backend::StringBackend;
use string_interner::{StringInterner, Symbol};

pub type Interner = StringInterner<StringBackend<Id>>;

#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Id(u64);

impl From<Id> for INT {
    fn from(value: Id) -> Self {
        value.0 as INT
    }
}

impl From<INT> for Id {
    fn from(value: INT) -> Self {
        Self(value as u64)
    }
}

impl Symbol for Id {
    fn try_from_usize(index: usize) -> Option<Self> {
        Some(Self(index as u64))
    }

    fn to_usize(self) -> usize {
        self.0 as usize
    }
}

pub fn id(a: &str, b: &str) -> IdRaw {
    IdRaw(SharedStr::from_ref(a), SharedStr::from_ref(b))
}

pub const fn id_static(a: &'static str, b: &'static str) -> IdRaw {
    IdRaw(SharedStr::from_static(a), SharedStr::from_static(b))
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct IdRaw(SharedStr, SharedStr);

pub static NONE: IdRaw = id_static("automancy", "none");

impl Display for IdRaw {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}:{}", self.0, self.1))
    }
}

impl IdRaw {
    pub fn to_id(&self, interner: &mut Interner) -> Id {
        interner.get_or_intern(self.to_string())
    }

    pub fn parse(s: &str) -> IdRaw {
        s.split_once(':')
            .map(|(a, b)| id(a, b))
            .unwrap_or_else(|| id("automancy", s))
    }
}

impl Serialize for IdRaw {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct IdRawVisitor;

impl<'de> Visitor<'de> for IdRawVisitor {
    type Value = IdRaw;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("an Id structured as namespace:name")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(IdRaw::parse(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(IdRaw::parse(&v))
    }
}

impl<'de> Deserialize<'de> for IdRaw {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(IdRawVisitor)
    }
}
