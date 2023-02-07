use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::game::item::{ItemAmount, ItemStackRaw};
use crate::resource::item::ItemRaw;
use crate::util::id::{Id, IdRaw, Interner};

#[derive(Debug, Default, Clone)]
pub struct Inventory(pub HashMap<Id, ItemAmount>);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InventoryRaw(pub Vec<ItemStackRaw>);

impl InventoryRaw {
    pub fn to_inventory(self, interner: &Interner) -> Inventory {
        Inventory(
            self.0
                .into_iter()
                .flat_map(|item| {
                    interner
                        .get(item.item.id.to_string())
                        .map(|id| (id, item.amount))
                })
                .collect(),
        )
    }

    pub fn from_inventory(inventory: Inventory, interner: &Interner) -> Self {
        Self(
            inventory
                .0
                .into_iter()
                .map(|(id, amount)| {
                    let id = IdRaw::parse(interner.resolve(id).unwrap());

                    let item = ItemRaw { id };

                    ItemStackRaw { item, amount }
                })
                .collect(),
        )
    }
}
