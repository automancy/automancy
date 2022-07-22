use std::mem::size_of_val;

use super::raw::{CanBeRaw, Raw};

pub trait Identifiable {
    fn id(&self) -> Id;
}

pub trait Comparable: Identifiable + PartialOrd<Id> {}

macro_rules! impl_cmp {
    ($Type: ty) => {
        impl crate::game::data::id::Comparable for $Type {}

        impl PartialEq<crate::game::data::id::Id> for $Type {
            fn eq(&self, other: &crate::game::data::id::Id) -> bool {
                self.id() == *other
            }
        }
    };
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id {
    pub namespace: String,
    pub name: String,
}

impl Id {
    pub fn automancy(name: String) -> Self {
        Self::new("automancy".to_string(), name)
    }

    pub fn new(namespace: String, name: String) -> Self {
        assert!(!namespace.contains(':'));
        assert!(!name.contains(':'));

        Self { namespace, name }
    }
}

impl CanBeRaw<RawId> for Id {}

#[derive(Debug, Clone)]
pub struct RawId {
    pub namespace: String,
    pub name: String,
}

impl Raw for RawId {
    fn to_bytes(self) -> Vec<u8> {
        let it = self.namespace.to_owned() + ":" + &self.name + "\0";
        it.into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let null = bytes.iter().position(|v| *v == b'\0').unwrap();

        let whole = String::from_utf8_lossy(&bytes[..null]);
        let split = whole.split(':').collect::<Vec<_>>();
        let (namespace, name) = (split[0].to_string(), split[1].to_string());

        Self { namespace, name }
    }

    fn convert(bytes: &[u8]) -> Vec<Self>
    where
        Self: Sized,
    {
        let len = bytes.len();

        let mut pos = 0;
        let mut vec = Vec::new();
        loop {
            if pos >= len {
                break;
            }

            let r = Self::from_bytes(&bytes[pos..]);
            pos += size_of_val(&r);

            vec.push(r);
        }

        vec
    }
}

impl Into<RawId> for Id {
    fn into(self) -> RawId {
        let namespace = self.namespace;
        let name = self.name;

        RawId { namespace, name }
    }
}

impl From<RawId> for Id {
    fn from(val: RawId) -> Self {
        let namespace = val.namespace;
        let name = val.name;

        Self { namespace, name }
    }
}
