use serde::{Deserialize, Serialize};

use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::Interner;

use crate::data::item::{Item, ItemRaw};
use crate::data::stack::ItemAmount;
use crate::ResourceManager;

#[derive(Debug, Default, Clone)]
pub struct Inventory(pub HashMap<Item, ItemAmount>);

impl Inventory {
    pub fn try_get(&self, item: Item) -> Option<ItemAmount> {
        self.0.get(&item).cloned()
    }

    pub fn get(&mut self, item: Item) -> ItemAmount {
        *self.0.entry(item).or_insert(0)
    }

    pub fn get_mut(&mut self, item: Item) -> &mut ItemAmount {
        self.0.entry(item).or_insert(0)
    }

    pub fn insert(&mut self, item: Item, amount: ItemAmount) {
        self.0.insert(item, amount);
    }

    pub fn take(&mut self, item: Item, amount: ItemAmount) -> Option<ItemAmount> {
        let stored = *self.0.get(&item)?;
        if stored == 0 {
            return None;
        }

        let taking = amount.min(stored);

        self.insert(item, stored - taking);

        Some(taking)
    }

    pub fn to_raw(&self, interner: &Interner) -> InventoryRaw {
        InventoryRaw(
            self.0
                .iter()
                .filter(|(_, amount)| **amount > 0)
                .flat_map(|(item, amount)| item.to_item_raw(interner).map(|item| (item, *amount)))
                .collect(),
        )
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InventoryRaw(pub Vec<(ItemRaw, ItemAmount)>);

impl InventoryRaw {
    pub fn to_inventory(&self, resource_man: &ResourceManager) -> Inventory {
        Inventory(
            self.0
                .iter()
                .flat_map(|(id, amount)| id.to_item(resource_man).map(|item| (item, *amount)))
                .collect(),
        )
    }
}
