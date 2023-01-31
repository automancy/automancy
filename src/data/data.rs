use std::collections::HashMap;

use crate::data::id::Id;
use crate::game::item::ItemAmount;

#[derive(Debug, Clone, Default)]
pub struct Data(pub HashMap<Id, ItemAmount>);
