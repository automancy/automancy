use crate::registry::Registry;
use crate::ResourceManager;
use automancy_defs::id::{Id, IdRaw, Interner};
use serde::{Deserialize, Serialize};

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

impl Item {
    pub fn to_item_raw(self, interner: &Interner) -> Option<ItemRaw> {
        let id = IdRaw::parse(interner.resolve(self.id)?);

        Some(ItemRaw(id))
    }
}

pub fn item_match(registry: &Registry, id: Id, other: Id) -> bool {
    if id == other {
        return true;
    }

    if let Some(tag) = registry.tags.get(&other) {
        return tag.of(registry, id);
    }

    false
}
