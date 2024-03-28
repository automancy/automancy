use crate::data::inventory::Inventory;
use crate::data::item::Item;
use crate::data::stack::{ItemAmount, ItemStack};
use crate::types::function::RhaiDataMap;
use crate::types::script::{Instructions, Script};
use crate::types::tag::Tag;
use crate::types::tile::TileDef;
use automancy_defs::id::Id;
use rhai::{Dynamic, Engine};

pub(crate) fn register_data_stuff(engine: &mut Engine) {
    engine
        .register_indexer_get_set(RhaiDataMap::rhai_get, RhaiDataMap::rhai_set)
        .register_fn(
            "get_or_new_inventory",
            RhaiDataMap::rhai_get_or_new_inventory,
        );

    engine
        .register_type_with_name::<Inventory>("Inventory")
        .register_fn("take", Inventory::take)
        .register_fn("take", Inventory::take_with_item)
        .register_fn("add", Inventory::add)
        .register_fn("add", Inventory::add_with_item)
        .register_indexer_get_set(Inventory::get, Inventory::insert)
        .register_indexer_get_set(Inventory::get_with_item, Inventory::insert_with_item);
    engine
        .register_type_with_name::<Id>("Id")
        .register_iterator::<Vec<Id>>();
    engine
        .register_type_with_name::<Script>("Script")
        .register_get("instructions", |v: &mut Script| v.instructions.clone());
    engine
        .register_type_with_name::<Instructions>("Instructions")
        .register_get("inputs", |v: &mut Instructions| match &v.inputs {
            Some(v) => Dynamic::from_iter(v.clone()),
            None => Dynamic::UNIT,
        })
        .register_get("outputs", |v: &mut Instructions| v.outputs.clone());
    engine.register_type_with_name::<TileDef>("Tile");
    engine
        .register_type_with_name::<Item>("Item")
        .register_iterator::<Vec<Item>>()
        .register_get("id", |v: &mut Item| v.id)
        .register_fn("==", |a: Item, b: Item| a == b)
        .register_fn("!=", |a: Item, b: Item| a != b);

    engine
        .register_type_with_name::<ItemStack>("ItemStack")
        .register_iterator::<Vec<ItemStack>>()
        .register_fn("ItemStack", |item: Item, amount: ItemAmount| ItemStack {
            item,
            amount,
        })
        .register_get("item", |v: &mut ItemStack| v.item)
        .register_get("amount", |v: &mut ItemStack| v.amount);
    engine.register_type_with_name::<Tag>("Tag");
}
