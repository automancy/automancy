use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::game::item::{ItemAmount, ItemRaw};
use crate::util::id::{Id, IdRaw, Interner};

#[derive(Debug, Default, Clone)]
pub struct Inventory(pub HashMap<Id, ItemAmount>);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InventoryRaw(pub Vec<ItemRaw>);

impl InventoryRaw {
    pub fn to_inventory(self, interner: &Interner) -> Inventory {
        Inventory(
            self.0
                .into_iter()
                .flat_map(|item| {
                    interner
                        .get(item.id.to_string())
                        .map(|id| (id, item.amount))
                })
                .collect(),
        )
    }

    pub fn from_inventory(data: Inventory, interner: &Interner) -> Self {
        Self(
            data.0
                .into_iter()
                .map(|(id, amount)| {
                    let id = IdRaw::parse(interner.resolve(id).unwrap());

                    ItemRaw { id, amount }
                })
                .collect(),
        )
    }
}
