use std::collections::HashMap;

use crate::game::item::ItemAmount;
use crate::util::id::Id;

#[derive(Debug, Clone, Default)]
pub struct Data(pub HashMap<Id, ItemAmount>);
