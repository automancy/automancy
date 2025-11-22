use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use crate::{
    game::generic::serailize::IdMap,
    id::{Id, Interner},
    math::Int,
};

pub type ItemAmount = Int;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStack {
    pub id: Id,
    pub amount: ItemAmount,
}

#[repr(transparent)]
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

    pub fn into_raw(self, id_map: &mut IdMap, interner: &Interner) -> serialize::InventoryRaw {
        let mut r = serialize::InventoryRaw::default();

        for (id, amount) in self.into_inner() {
            id_map.insert(id, interner);

            r.push((id, amount));
        }

        r
    }
}

pub mod serialize {
    use core::ops::{Deref, DerefMut};

    use serde::{Deserialize, Serialize};

    use crate::{
        game::{
            generic::serailize::{IdMap, IdMapError},
            inventory::{Inventory, ItemAmount},
        },
        id::{Id, Interner},
    };

    #[repr(transparent)]
    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct InventoryRaw(Vec<(Id, ItemAmount)>);

    impl Deref for InventoryRaw {
        type Target = Vec<(Id, ItemAmount)>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for InventoryRaw {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl From<Vec<(Id, ItemAmount)>> for InventoryRaw {
        fn from(value: Vec<(Id, ItemAmount)>) -> Self {
            Self(value)
        }
    }

    impl InventoryRaw {
        pub fn into_inner(self) -> Vec<(Id, ItemAmount)> {
            self.0
        }

        pub fn into_inventory(self, id_map: &IdMap, interner: &Interner) -> Result<Inventory, IdMapError> {
            let mut r = Inventory::default();

            for (unmapped_id, amount) in self.0.into_iter() {
                r.insert(id_map.resolve(unmapped_id, interner)?, amount);
            }

            Ok(r)
        }
    }
}

pub mod deserialize {
    use core::ops::{Deref, DerefMut};

    use serde::Deserialize;
    use serde_tuple::Deserialize_tuple;

    use crate::{
        game::inventory::{Inventory, ItemAmount},
        id::{
            Interner,
            deserialize::{StrId, StrIdParseError},
        },
    };

    #[derive(Debug, PartialEq, Eq, Deserialize_tuple)]
    pub struct ItemStackStr {
        pub id: StrId,
        pub amount: ItemAmount,
    }

    #[repr(transparent)]
    #[derive(Debug, Default, Clone, Deserialize)]
    #[serde(transparent)]
    pub struct InventoryStr(Vec<(StrId, ItemAmount)>);

    impl Deref for InventoryStr {
        type Target = Vec<(StrId, ItemAmount)>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for InventoryStr {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl From<Vec<(StrId, ItemAmount)>> for InventoryStr {
        fn from(value: Vec<(StrId, ItemAmount)>) -> Self {
            Self(value)
        }
    }

    impl InventoryStr {
        pub fn into_inner(self) -> Vec<(StrId, ItemAmount)> {
            self.0
        }

        pub fn into_inventory(self, interner: &mut Interner, fallback_namespace: Option<&str>) -> Result<Inventory, StrIdParseError> {
            let mut r = Inventory::default();

            for (id, amount) in self.0.into_iter() {
                r.insert(id.into_id(interner, fallback_namespace)?, amount);
            }

            Ok(r)
        }
    }
}
