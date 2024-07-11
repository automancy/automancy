use hashbrown::{HashMap, HashSet};
use rhai::{Dynamic, Engine};

use automancy_defs::{coord::TileCoord, stack::ItemStack};
use automancy_defs::{id::Id, stack::ItemAmount};

use crate::types::tag::TagDef;
use crate::types::tile::TileDef;
use crate::types::{
    item::ItemDef,
    script::{InstructionsDef, ScriptDef},
};
use crate::{data::DataMap, inventory::Inventory};

pub(crate) fn register_data_stuff(engine: &mut Engine) {
    engine
        .register_type_with_name::<DataMap>("DataMap")
        .register_indexer_get_set(DataMap::rhai_get, DataMap::rhai_set)
        .register_fn("get_or_new_inventory", DataMap::get_or_new_inventory);

    engine
        .register_type_with_name::<Inventory>("Inventory")
        .register_fn("take", Inventory::take)
        .register_fn("add", Inventory::add)
        .register_indexer_get_set(Inventory::get, Inventory::insert);

    engine
        .register_type_with_name::<Id>("Id")
        .register_fn("==", |a: Id, b: Id| a == b)
        .register_fn("!=", |a: Id, b: Id| a != b)
        .register_fn("contains", |v: &mut HashSet<Id>, id: Id| -> bool {
            v.contains(&id)
        });

    engine
        .register_type_with_name::<ItemStack>("ItemStack")
        .register_fn("ItemStack", |id: Id, amount: ItemAmount| -> ItemStack {
            ItemStack { id, amount }
        })
        .register_fn("ItemStack", |amount: ItemAmount, id: Id| -> ItemStack {
            ItemStack { id, amount }
        })
        .register_get("id", |v: &mut ItemStack| -> Id { v.id })
        .register_get("amount", |v: &mut ItemStack| -> ItemAmount { v.amount });

    engine
        .register_type_with_name::<HashMap<TileCoord, Id>>("TileMap")
        .register_indexer_get(
            |v: &mut HashMap<TileCoord, Id>, coord: TileCoord| -> Dynamic {
                if let Some(v) = v.get(&coord).copied() {
                    Dynamic::from(v)
                } else {
                    Dynamic::UNIT
                }
            },
        )
        .register_indexer_set(|v: &mut HashMap<TileCoord, Id>, coord: TileCoord, id: Id| {
            v.insert(coord, id);
        })
        .register_fn(
            "contains",
            |v: &mut HashMap<TileCoord, Id>, coord: TileCoord| -> bool { v.contains_key(&coord) },
        )
        .register_fn("keys", |v: &mut HashMap<TileCoord, Id>| -> Dynamic {
            Dynamic::from_iter(v.keys().cloned())
        })
        .register_fn("TileMap", HashMap::<TileCoord, Id>::new)
        .register_fn(
            "TileMap",
            |v: Vec<(TileCoord, Id)>| -> HashMap<TileCoord, Id> {
                HashMap::<TileCoord, Id>::from_iter(v)
            },
        );

    engine
        .register_type_with_name::<HashMap<Id, HashSet<Id>>>("MapSetId")
        .register_indexer_get(|v: &mut HashMap<Id, HashSet<Id>>, id: Id| -> Dynamic {
            if let Some(v) = v.get(&id).cloned() {
                Dynamic::from_iter(v)
            } else {
                Dynamic::UNIT
            }
        })
        .register_fn("keys", |v: HashMap<Id, HashSet<Id>>| -> Dynamic {
            Dynamic::from_iter(v.into_keys())
        });

    engine
        .register_type_with_name::<ItemDef>("ItemDef")
        .register_get("id", |v: &mut ItemDef| -> Id { v.id })
        .register_fn("==", |a: ItemDef, b: ItemDef| a == b)
        .register_fn("!=", |a: ItemDef, b: ItemDef| a != b);
    engine
        .register_type_with_name::<ScriptDef>("ScriptDef")
        .register_get("instructions", |v: &mut ScriptDef| -> InstructionsDef {
            v.instructions.clone()
        });
    engine
        .register_type_with_name::<InstructionsDef>("InstructionsDef")
        .register_get("inputs", |v: &mut InstructionsDef| -> Dynamic {
            if let Some(v) = &v.inputs {
                Dynamic::from_iter(v.iter().cloned())
            } else {
                Dynamic::UNIT
            }
        })
        .register_get("outputs", |v: &mut InstructionsDef| -> Dynamic {
            Dynamic::from_iter(v.outputs.iter().cloned())
        });
    engine.register_type_with_name::<TileDef>("TileDef");
    engine.register_type_with_name::<TagDef>("TagDef");
}
