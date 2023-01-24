use std::fmt::{self, Display};

use flexstr::{SharedStr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]

pub struct Id(SharedStr, SharedStr);

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.0, self.1)
    }
}


pub fn id(a: &str, b: &str) -> Id {
    Id(SharedStr::from(a), SharedStr::from(b))
}

pub const fn id_static(a: &'static str, b: &'static str) -> Id {
    Id(SharedStr::from_static(a), SharedStr::from_static(b))
}

pub fn id_parse(s: &str) -> Option<Id> {
    s.split_once(':').map(|(a, b)| id(a, b))
}
