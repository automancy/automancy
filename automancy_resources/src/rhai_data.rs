use hashbrown::{HashMap, HashSet};
use rhai::{Dynamic, Engine};

use automancy_defs::coord::TileCoord;
use automancy_defs::id::Id;

use crate::data::inventory::Inventory;
use crate::data::item::Item;
use crate::data::stack::{ItemAmount, ItemStack};
use crate::types::function::RhaiDataMap;
use crate::types::script::{Instructions, Script};
use crate::types::tag::Tag;
use crate::types::tile::TileDef;

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
        .register_iterator::<Vec<Id>>()
        .register_iterator::<HashSet<Id>>()
        .register_fn("contains", |v: &mut HashSet<Id>, id: Dynamic| {
            if let Ok(id) = id.as_int() {
                Dynamic::from_bool(v.contains(&Id::from(id)))
            } else {
                Dynamic::UNIT
            }
        });
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

    engine
        .register_type_with_name::<HashMap<TileCoord, Id>>("TileMap")
        .register_indexer_get_set(
            |v: &mut HashMap<TileCoord, Id>, coord: TileCoord| {
                if let Some(id) = v.get(&coord) {
                    Dynamic::from_int((*id).into())
                } else {
                    Dynamic::UNIT
                }
            },
            |v: &mut HashMap<TileCoord, Id>, coord: TileCoord, id: Dynamic| {
                if let Ok(id) = id.as_int() {
                    v.insert(coord, Id::from(id));
                } else if let Some(id) = id.try_cast::<Id>() {
                    v.insert(coord, id);
                }
            },
        )
        .register_fn(
            "contains",
            |v: &mut HashMap<TileCoord, Id>, coord: TileCoord| {
                Dynamic::from_bool(v.contains_key(&coord))
            },
        )
        .register_fn("keys", |v: &mut HashMap<TileCoord, Id>| {
            Dynamic::from_iter(v.keys().cloned())
        })
        .register_fn("TileMap", HashMap::<TileCoord, Id>::new)
        .register_fn("TileMap", |v: Vec<(TileCoord, Id)>| {
            HashMap::<TileCoord, Id>::from_iter(v)
        });

    engine
        .register_type_with_name::<HashMap<Id, HashSet<Id>>>("MapSetId")
        .register_indexer_get(|v: &mut HashMap<Id, HashSet<Id>>, id: Dynamic| {
            if let Ok(id) = id.as_int() {
                if let Some(v) = v.get(&Id::from(id)).cloned() {
                    Dynamic::from(v)
                } else {
                    Dynamic::UNIT
                }
            } else {
                Dynamic::UNIT
            }
        })
        .register_fn("keys", |v: HashMap<Id, HashSet<Id>>| {
            Dynamic::from_iter(v.into_keys())
        });
}
