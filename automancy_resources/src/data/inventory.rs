use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use automancy_defs::id::{Id, IdRaw, Interner};

use crate::data::item::Item;
use crate::data::stack::ItemAmount;
use crate::ResourceManager;

#[derive(Debug, Default, Clone)]
pub struct Inventory(pub BTreeMap<Id, ItemAmount>);

impl Inventory {
    pub fn get(&mut self, id: Id) -> ItemAmount {
        *self.0.entry(id).or_insert(0)
    }

    pub fn get_mut(&mut self, id: Id) -> &mut ItemAmount {
        self.0.entry(id).or_insert(0)
    }

    pub fn insert(&mut self, id: Id, amount: ItemAmount) {
        self.0.insert(id, amount);
    }

    pub fn add(&mut self, id: Id, amount: ItemAmount) {
        let stored = self.0.get(&id).cloned().unwrap_or(0);

        self.0.insert(id, stored + amount);
    }

    pub fn take(&mut self, id: Id, amount: ItemAmount) -> Option<ItemAmount> {
        let stored = *self.0.get(&id)?;
        if stored == 0 {
            return None;
        }

        let taking = amount.min(stored);

        self.insert(id, stored - taking);

        Some(taking)
    }

    pub fn get_with_item(&mut self, item: Item) -> ItemAmount {
        self.get(item.id)
    }

    pub fn get_mut_with_item(&mut self, item: Item) -> &mut ItemAmount {
        self.get_mut(item.id)
    }

    pub fn insert_with_item(&mut self, item: Item, amount: ItemAmount) {
        self.insert(item.id, amount)
    }

    pub fn add_with_item(&mut self, item: Item, amount: ItemAmount) {
        self.add(item.id, amount)
    }

    pub fn take_with_item(&mut self, item: Item, amount: ItemAmount) -> Option<ItemAmount> {
        self.take(item.id, amount)
    }

    pub fn to_raw(&self, interner: &Interner) -> InventoryRaw {
        InventoryRaw(
            self.0
                .iter()
                .filter(|(_, amount)| **amount > 0)
                .flat_map(|(item, amount)| {
                    interner
                        .resolve(*item)
                        .map(|item| (IdRaw::parse(item), *amount))
                })
                .collect(),
        )
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InventoryRaw(pub Vec<(IdRaw, ItemAmount)>);

impl InventoryRaw {
    pub fn to_inventory(&self, resource_man: &ResourceManager) -> Inventory {
        Inventory(
            self.0
                .iter()
                .flat_map(|(id, amount)| {
                    resource_man
                        .interner
                        .get(id.to_string())
                        .map(|item| (item, *amount))
                })
                .collect(),
        )
    }
}
