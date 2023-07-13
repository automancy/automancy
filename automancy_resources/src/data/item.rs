use std::cmp::Ordering;

use rhai::Dynamic;

use automancy_defs::id::Id;

use crate::data::stack::ItemStack;
use crate::{ResourceManager, RESOURCE_MAN};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Item {
    pub id: Id,
    pub model: Id,
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

pub fn item_match(resource_man: &ResourceManager, id: Id, other: Id) -> bool {
    if let Some(tag) = resource_man.registry.tags.get(&other) {
        return tag.of(&resource_man.registry, id);
    }

    if id == other {
        return true;
    }

    false
}

pub fn rhai_item_match(id: Id, other: Id) -> bool {
    item_match(RESOURCE_MAN.read().unwrap().as_ref().unwrap(), id, other)
}

pub fn item_matches(
    resource_man: &ResourceManager,
    id: Id,
    mut others: impl Iterator<Item = Item>,
) -> Option<Item> {
    others.find(|&other| item_match(resource_man, id, other.id))
}

pub fn item_stack_matches(
    resource_man: &ResourceManager,
    id: Id,
    mut others: impl Iterator<Item = ItemStack>,
) -> Option<ItemStack> {
    others.find(|&other| item_match(resource_man, id, other.item.id))
}

pub fn rhai_item_matches(id: Id, others: rhai::Array) -> Dynamic {
    match item_matches(
        RESOURCE_MAN.read().unwrap().as_ref().unwrap(),
        id,
        others.into_iter().map(|v| v.cast::<Item>()),
    ) {
        Some(v) => Dynamic::from(v),
        None => Dynamic::UNIT,
    }
}

pub fn rhai_item_stack_matches(id: Id, others: rhai::Array) -> Dynamic {
    match item_stack_matches(
        RESOURCE_MAN.read().unwrap().as_ref().unwrap(),
        id,
        others.into_iter().map(|v| v.cast::<ItemStack>()),
    ) {
        Some(v) => Dynamic::from(v),
        None => Dynamic::UNIT,
    }
}

/*
pub fn item_match_str(resource_man: &ResourceManager, id: Id, other: &str) -> bool {
    if let Some(other) = resource_man.interner.get(other) {
        item_match(resource_man, id, other)
    } else {
        false
    }
}

pub fn rhai_item_match_str(id: Id, other: ImmutableString) -> bool {
    item_match_str(
        RESOURCE_MAN.read().unwrap().as_ref().unwrap(),
        id,
        other.as_str(),
    )
}
 */
