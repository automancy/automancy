use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use crate::{
    game::item::{ItemAmount, ItemStack},
    id::{Id, Interner},
    parse::id::resolve_map_id_item,
};

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

    pub fn to_raw(self, interner: &Interner) -> raw::InventoryRaw {
        raw::InventoryRaw(
            resolve_map_id_item(
                self.0
                    .into_iter()
                    .filter(|(_, amount)| *amount > 0)
                    .map(|(a, b)| (a, b)),
                interner,
            )
            .collect(),
        )
    }
}

pub mod raw {
    use serde::{Deserialize, Serialize};

    use crate::{
        game::{inventory::Inventory, item::ItemAmount},
        id::Interner,
        parse::id::{parse_map_id_item, try_parse_map_id_item},
    };

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct InventoryRaw(pub(crate) Vec<(String, ItemAmount)>);

    impl InventoryRaw {
        pub fn try_to_inventory(self, interner: &Interner) -> Inventory {
            Inventory(
                try_parse_map_id_item(self.0.into_iter().map(|(a, b)| (a, b)), interner).collect(),
            )
        }

        pub fn to_inventory(
            self,
            interner: &mut Interner,
            namespace: Option<impl AsRef<str>>,
        ) -> Inventory {
            Inventory(
                parse_map_id_item(self.0.into_iter().map(|(a, b)| (a, b)), interner, namespace)
                    .collect(),
            )
        }
    }
}
