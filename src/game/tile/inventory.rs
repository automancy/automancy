use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::game::item::ItemAmount;
use crate::resource::item::Item;
use crate::resource::ResourceManager;
use crate::util::id::{IdRaw, Interner};

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
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InventoryRaw(pub Vec<(IdRaw, ItemAmount)>);

impl InventoryRaw {
    pub fn intern_to_inventory(self, resource_man: &mut ResourceManager) -> Inventory {
        Inventory(
            self.0
                .into_iter()
                .flat_map(|(id, amount)| {
                    resource_man
                        .registry
                        .item(id.to_id(&mut resource_man.interner))
                        .cloned()
                        .map(|item| (item, amount))
                })
                .collect(),
        )
    }

    pub fn to_inventory(self, resource_man: &ResourceManager) -> Inventory {
        Inventory(
            self.0
                .into_iter()
                .flat_map(|(id, amount)| {
                    resource_man.interner.get(id.to_string()).and_then(|id| {
                        resource_man
                            .registry
                            .item(id)
                            .cloned()
                            .map(|item| (item, amount))
                    })
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
                .flat_map(|(item, amount)| {
                    interner
                        .resolve(item.id)
                        .map(IdRaw::parse)
                        .map(|id| (id, amount))
                })
                .collect(),
        )
    }
}
