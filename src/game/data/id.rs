use serde::{Deserialize, Serialize};

use super::id_pool::IdPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Id {
    pub namespace: usize,
    pub name: usize,
}

impl Default for Id {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug)]
pub enum IdError {
    NotFound,
    ParseError(Option<RawId>, &'static str),
}

impl Id {
    pub fn new(namespace: usize, name: usize) -> Self {
        Self { namespace, name }
    }

    pub fn none() -> Self {
        Self::new(0, 0)
    }

    pub fn automancy(name: usize) -> Self {
        Self::new(0, name)
    }

    pub fn from_str(id_pool: &mut IdPool, string: &str) -> Result<Self, IdError> {
        let (namespace, name) = string.split_once(':').ok_or(IdError::ParseError(
            None,
            "invalid id: no delimiter ':' found",
        ))?;

        let raw_id = RawId::new(namespace, name);

        Self::from_raw_id(id_pool, raw_id)
    }

    pub fn from_raw_id(id_pool: &mut IdPool, raw_id: RawId) -> Result<Self, IdError> {
        if raw_id.namespace.contains(':') {
            return Err(IdError::ParseError(
                Some(raw_id),
                "namespace contained ':' ... this must mean something is horribly wrong.",
            ));
        }

        if raw_id.name.contains(':') {
            return Err(IdError::ParseError(
                Some(raw_id),
                "name contained ':', please change the name to not have ':' in it!",
            ));
        }

        Ok(raw_id.to_id_mut(id_pool))
    }

    pub fn to_raw_id(self, id_pool: &IdPool) -> RawId {
        id_pool.raw_id(self)
    }
}

#[derive(Debug, Clone)]
pub struct RawId {
    pub namespace: String,
    pub name: String,
}

pub const AUTOMANCY_NAMESPACE: &str = "automancy";
pub const NONE_NAME: &str = "none";

impl RawId {
    pub fn none() -> Self {
        Self {
            namespace: AUTOMANCY_NAMESPACE.to_owned(),
            name: NONE_NAME.to_owned(),
        }
    }

    pub fn new(namespace: &str, name: &str) -> Self {
        Self {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
        }
    }

    pub fn to_id(self, id_pool: &IdPool) -> Option<Id> {
        id_pool.id(&self)
    }

    pub fn to_id_mut(self, id_pool: &mut IdPool) -> Id {
        id_pool.id_mut(&self)
    }
}
