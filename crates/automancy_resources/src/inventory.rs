use automancy_defs::{
    id::{Id, Interner},
    parse_map_id_of, resolve_map_id_of,
    stack::{ItemAmount, ItemStack},
    try_parse_map_id_of,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Inventory(BTreeMap<Id, ItemAmount>);

impl Deref for Inventory {
    type Target = BTreeMap<Id, ItemAmount>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Inventory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Inventory {
    pub fn into_inner(self) -> BTreeMap<Id, ItemAmount> {
        self.0
    }

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
        let stored = self.get(id);

        self.insert(id, stored + amount);
    }

    pub fn contains(&mut self, stack: ItemStack) -> bool {
        self.get(stack.id) >= stack.amount
    }

    pub fn take(&mut self, id: Id, amount: ItemAmount) -> ItemAmount {
        let stored = self.get(id);
        let taking = amount.min(stored);

        self.insert(id, stored - taking);

        taking
    }

    pub fn to_raw(&self, interner: &Interner) -> InventoryRaw {
        InventoryRaw(resolve_map_id_of(
            self.0
                .iter()
                .filter(|(_, amount)| **amount > 0)
                .map(|(a, b)| (*a, *b)),
            interner,
        ))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InventoryRaw(Vec<(String, ItemAmount)>);

impl InventoryRaw {
    pub fn try_to_inventory(&self, interner: &Interner) -> Inventory {
        Inventory(try_parse_map_id_of(
            self.0.iter().map(|(a, b)| (a, *b)),
            interner,
        ))
    }

    pub fn to_inventory(&self, interner: &mut Interner, namespace: Option<&str>) -> Inventory {
        Inventory(parse_map_id_of(
            self.0.iter().map(|(a, b)| (a, *b)),
            interner,
            namespace,
        ))
    }
}
