use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::data::id::Id;
use crate::game::item::ItemAmount;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Data(pub HashMap<Id, ItemAmount>);
