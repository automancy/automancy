use rhai::plugin::*;
use rhai::Module;
use rhai::{exported_module, Engine};

#[export_module]
mod utils {
    use automancy_defs::{id::Id, stack::ItemStack};
    use rhai::Array;

    use crate::{types::item::ItemDef, RESOURCE_MAN};

    pub fn item_match(id: Id, other: Id) -> bool {
        crate::item_match(RESOURCE_MAN.read().unwrap().as_ref().unwrap(), id, other)
    }

    pub fn item_matches(id: Id, others: Array) -> Dynamic {
        match crate::item_matches(
            RESOURCE_MAN.read().unwrap().as_ref().unwrap(),
            id,
            others.into_iter().map(|v| v.cast::<ItemDef>()),
        ) {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    }

    pub fn item_stack_matches(id: Id, others: Array) -> Dynamic {
        match crate::item_stack_matches(
            RESOURCE_MAN.read().unwrap().as_ref().unwrap(),
            id,
            others.into_iter().map(|v| v.cast::<ItemStack>()),
        ) {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    }

    pub fn item_ids_of_tag(id: Id) -> Dynamic {
        Dynamic::from_iter(crate::item_ids_of_tag(
            RESOURCE_MAN.read().unwrap().as_ref().unwrap(),
            id,
        ))
    }
}

pub(crate) fn register_functions(engine: &mut Engine) {
    engine.register_global_module(exported_module!(utils).into());
}
