use std::collections::HashMap;

use rune::Any;
use serde::{Deserialize, Serialize};

use crate::game::item::{ItemAmount, ItemStackRaw};
use crate::resource::item::ItemRaw;
use crate::util::id::{Id, IdRaw, Interner};

#[derive(Debug, Default, Clone, Any)]
pub struct Inventory(pub HashMap<Id, ItemAmount>);

impl Inventory {
    pub fn get(&mut self, id: Id) -> ItemAmount {
        *self.0.entry(id).or_insert(0)
    }

    pub fn insert(&mut self, id: Id, amount: ItemAmount) {
        self.0.insert(id, amount);
    }
    
    pub fn take(&mut self, id: Id, amount: ItemAmount) -> Option<ItemAmount> {
        let stored = *self.0.get(&id)?;
        let taking = amount.min(stored);
        
        self.insert(id, stored - taking);
        
        Some(taking)
    }
}

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
                .filter(|(_, amount)| *amount > 0)
                .map(|(id, amount)| {
                    let id = IdRaw::parse(interner.resolve(id).unwrap());

                    let item = ItemRaw { id };

                    ItemStackRaw { item, amount }
                })
                .collect(),
        )
    }
}
