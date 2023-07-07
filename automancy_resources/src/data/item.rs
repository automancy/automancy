use std::cmp::Ordering;

use rhai::Dynamic;
use serde::{Deserialize, Serialize};

use automancy_defs::id::{Id, IdRaw, Interner};

use crate::ResourceManager;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ItemRaw(pub IdRaw);

impl ItemRaw {
    pub fn to_item(&self, resource_man: &ResourceManager) -> Option<Item> {
        let id = resource_man.interner.get(self.0.to_string())?;

        resource_man.registry.item(id).cloned()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Item {
    pub id: Id,
    pub model: Id,
}

impl From<Item> for Dynamic {
    fn from(value: Item) -> Self {
        Dynamic::from_int(value.id.into())
    }
}

impl PartialOrd<Self> for Item {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Item {
    pub fn to_item_raw(self, interner: &Interner) -> Option<ItemRaw> {
        let id = IdRaw::parse(interner.resolve(self.id)?);

        Some(ItemRaw(id))
    }
}

pub fn item_match(resource_man: &ResourceManager, id: Id, other: Id) -> bool {
    if let Some(tag) = resource_man.registry.tags.get(&other) {
        return tag.of(&resource_man.registry, id);
    }

    if id == other {
        return true;
    }

    false
}

pub fn item_match_str(resource_man: &ResourceManager, id: Id, other: &str) -> bool {
    if let Some(other) = resource_man.interner.get(other) {
        item_match(resource_man, id, other)
    } else {
        false
    }
}
