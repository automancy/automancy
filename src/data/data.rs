use serde::{Deserialize, Serialize};

pub type SingleData = u32;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Data(pub Vec<SingleData>);
