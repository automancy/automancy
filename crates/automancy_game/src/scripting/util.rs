use automancy_data::{
    game::inventory::ItemStack,
    id::{Id, ItemId, TagId},
    id_map::IdSet,
};
use rhai::{Engine, exported_module, plugin::*};

use crate::resources::{ResourceManager, types::item::ItemDef};

fn item_match(resource_man: &ResourceManager, id: ItemId, other: Id) -> bool {
    if id.is_none() {
        return false;
    }

    if let Some(tag) = resource_man.registry.tag_defs.get(&TagId(other)) {
        return tag.contains(*id);
    }

    if *id == other {
        return true;
    }

    false
}

fn item_matches(resource_man: &ResourceManager, id: ItemId, mut others: impl Iterator<Item = ItemDef>) -> Option<ItemDef> {
    others.find(|&other| item_match(resource_man, id, *other.id))
}

fn item_stack_matches(resource_man: &ResourceManager, id: ItemId, mut others: impl Iterator<Item = ItemStack>) -> Option<ItemStack> {
    others.find(|&other| item_match(resource_man, id, *other.id))
}

fn item_ids_of_tag(resource_man: &ResourceManager, id: Id) -> IdSet<ItemId> {
    resource_man
        .ordered_items
        .iter()
        .filter(|v| item_match(resource_man, **v, id))
        .cloned()
        .collect()
}

#[export_module]
mod utils {
    use rhai::Array;

    use crate::resources;

    pub fn item_match(id: ItemId, other: Id) -> bool {
        super::item_match(&resources::global::resource_man(), id, other)
    }

    pub fn item_matches(id: ItemId, others: Array) -> Dynamic {
        match super::item_matches(
            &resources::global::resource_man(),
            id,
            others.into_iter().map(|v| v.cast::<ItemDef>()),
        ) {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    }

    pub fn item_stack_matches(id: ItemId, others: Array) -> Dynamic {
        match super::item_stack_matches(
            &resources::global::resource_man(),
            id,
            others.into_iter().map(|v| v.cast::<ItemStack>()),
        ) {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    }

    pub fn item_ids_of_tag(id: Id) -> Dynamic {
        Dynamic::from(super::item_ids_of_tag(&resources::global::resource_man(), id))
    }
}

pub(crate) fn register_script_stuff(engine: &mut Engine) {
    engine.register_global_module(exported_module!(utils).into());
}
