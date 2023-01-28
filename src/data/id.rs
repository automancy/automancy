use std::fmt::{self, Display, Formatter};

use flexstr::{SharedStr};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, Visitor};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(SharedStr, SharedStr);

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.0, self.1)
    }
}

pub fn id(a: &str, b: &str) -> Id {
    Id(SharedStr::from(a), SharedStr::from(b))
}

pub const fn id_static(a: &'static str, b: &'static str) -> Id {
    Id(SharedStr::from_static(a), SharedStr::from_static(b))
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

struct IdVisitor;

impl<'de> Visitor<'de> for IdVisitor {
    type Value = Id;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("an Id structured as namespace:name")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
        Ok(Id::parse(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: Error {
        Ok(Id::parse(&v))
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_str(IdVisitor)
    }
}

impl Id {
    pub fn parse(s: &str) -> Id {
        s.split_once(':')
            .map(|(a, b)| id(a, b))
            .unwrap_or_else(|| id("automancy", s))
    }
}