use core::ops::Neg;
use std::{
    collections::{BTreeMap, btree_map},
    ops::{Deref, DerefMut},
};

use crate::{id::ItemId, math::Int};

pub type ItemAmount = Int;

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStack {
    pub id: ItemId,
    pub amount: ItemAmount,
}

impl Neg for ItemStack {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            id: self.id,
            amount: -self.amount,
        }
    }
}

impl From<(ItemId, ItemAmount)> for ItemStack {
    fn from((id, amount): (ItemId, ItemAmount)) -> Self {
        Self {
            id,
            amount,
        }
    }
}

#[must_use]
#[repr(transparent)]
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Inventory(BTreeMap<ItemId, ItemAmount>);

impl Deref for Inventory {
    type Target = BTreeMap<ItemId, ItemAmount>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Inventory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<(ItemId, ItemAmount)> for Inventory {
    fn from_iter<T: IntoIterator<Item = (ItemId, ItemAmount)>>(iter: T) -> Self {
        Self(BTreeMap::from_iter(iter))
    }
}

impl FromIterator<ItemStack> for Inventory {
    fn from_iter<T: IntoIterator<Item = ItemStack>>(iter: T) -> Self {
        Self(BTreeMap::from_iter(iter.into_iter().map(|stack| (stack.id, stack.amount))))
    }
}

impl IntoIterator for Inventory {
    type Item = (ItemId, ItemAmount);
    type IntoIter = btree_map::IntoIter<ItemId, ItemAmount>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Inventory {
    pub fn into_inner(self) -> BTreeMap<ItemId, ItemAmount> {
        self.0
    }

    pub fn into_stacks(self) -> impl Iterator<Item = ItemStack> {
        self.into_iter().map(ItemStack::from)
    }

    pub fn get(&self, id: ItemId) -> ItemAmount {
        self.0.get(&id).copied().unwrap_or(0)
    }

    pub fn get_mut(&mut self, id: ItemId) -> &mut ItemAmount {
        self.0.entry(id).or_insert(0)
    }

    pub fn set(&mut self, id: ItemId, amount: ItemAmount) {
        self.0.insert(id, amount);
    }

    pub fn contains(&self, stack: ItemStack) -> bool {
        self.get(stack.id) >= stack.amount
    }

    pub fn add(&mut self, id: ItemId, amount: ItemAmount) {
        *self.get_mut(id) += amount;
    }

    pub fn take(&mut self, id: ItemId, amount: ItemAmount) -> ItemAmount {
        let stored = self.get_mut(id);
        let taking = amount.min(*stored);

        *stored -= taking;

        taking
    }
}

pub mod serialize {
    use core::ops::{Deref, DerefMut};

    use serde::{Deserialize, Serialize};

    use crate::{
        game::{
            generic::serailize::{IdMapping, IdMappingError},
            inventory::{Inventory, ItemAmount},
        },
        id::{Id, IdInterner, ItemId},
    };

    #[must_use]
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

        pub fn into_inventory(self, id_map: &IdMapping, interner: &IdInterner) -> Result<Inventory, IdMappingError> {
            let mut r = Inventory::default();

            for (unmapped_id, amount) in self.0.into_iter() {
                r.set(ItemId(id_map.resolve(unmapped_id, interner)?), amount);
            }

            Ok(r)
        }
    }

    impl Inventory {
        pub fn into_raw(self, id_map: &mut IdMapping, interner: &IdInterner) -> InventoryRaw {
            let mut r = InventoryRaw::default();

            for (id, amount) in self {
                id_map.insert(*id, interner);

                r.push((*id, amount));
            }

            r
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
            IdInterner, ItemId,
            deserialize::{StrId, StrIdParseError},
        },
    };

    #[must_use]
    #[derive(Debug, PartialEq, Eq, Deserialize_tuple)]
    pub struct ItemStackStr {
        pub id: StrId,
        pub amount: ItemAmount,
    }

    #[must_use]
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

        pub fn into_inventory(self, interner: &mut IdInterner, fallback_namespace: Option<&str>) -> Result<Inventory, StrIdParseError> {
            let mut r = Inventory::default();

            for (id, amount) in self.0.into_iter() {
                r.set(ItemId(interner.get_or_intern(id, fallback_namespace)?), amount);
            }

            Ok(r)
        }
    }
}
