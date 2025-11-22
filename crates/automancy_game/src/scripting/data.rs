use core::any::TypeId;

use automancy_data::{
    game::{
        coord::{TileBounds, TileCoord},
        generic::{DataMap, Datum},
        inventory::{Inventory, ItemAmount, ItemStack},
    },
    id::{Id, ModelId, TileId},
};
use hashbrown::{HashMap, HashSet};
use rhai::{Dynamic, Engine};

use crate::{
    resources,
    resources::types::{item::ItemDef, recipe::RecipeDef, tag::TagDef, tile::TileDef},
};

pub fn data_into_dynamic(v: Datum) -> Dynamic {
    match v {
        Datum::Inventory(v) => Dynamic::from(v),
        Datum::Coord(v) => Dynamic::from(v),
        Datum::VecCoord(v) => Dynamic::from_iter(v),
        Datum::TileBounds(v) => Dynamic::from(v),
        Datum::Id(v) => Dynamic::from(v),
        Datum::Color(v) => Dynamic::from(v),
        Datum::VecId(v) => Dynamic::from_iter(v),
        Datum::SetId(v) => Dynamic::from_iter(v),
        Datum::Amount(v) => Dynamic::from_int(v),
        Datum::Bool(v) => Dynamic::from_bool(v),
        Datum::TileMap(v) => Dynamic::from(v),
        Datum::MapSetId(v) => Dynamic::from(v),
    }
}

pub fn data_from_dynamic(v: Dynamic) -> Option<Datum> {
    let id = v.type_id();

    Some(if id == TypeId::of::<TileCoord>() {
        Datum::Coord(v.cast())
    } else if id == TypeId::of::<Id>() {
        Datum::Id(v.cast())
    } else if id == TypeId::of::<ItemAmount>() {
        Datum::Amount(v.cast())
    } else if id == TypeId::of::<bool>() {
        Datum::Bool(v.cast())
    } else if id == TypeId::of::<Inventory>() {
        Datum::Inventory(v.cast())
    } else if id == TypeId::of::<Vec<TileCoord>>() {
        Datum::VecCoord(v.cast())
    } else if id == TypeId::of::<Vec<Id>>() {
        Datum::VecId(v.cast())
    } else if id == TypeId::of::<HashSet<Id>>() {
        Datum::SetId(v.cast())
    } else if id == TypeId::of::<TileBounds>() {
        Datum::TileBounds(v.cast())
    } else if id == TypeId::of::<HashMap<TileCoord, Id>>() {
        Datum::TileMap(v.cast())
    } else if id == TypeId::of::<HashMap<Id, HashSet<Id>>>() {
        Datum::MapSetId(v.cast())
    } else {
        return None;
    })
}

pub fn rhai_get(data: &mut DataMap, id: Id) -> Dynamic {
    if let Some(datum) = data.get(id).cloned() {
        data_into_dynamic(datum)
    } else {
        Dynamic::UNIT
    }
}

pub fn rhai_set(data: &mut DataMap, id: Id, dynamic: Dynamic) {
    if let Some(datum) = data_from_dynamic(dynamic) {
        data.set(id, datum);
    }
}

pub(crate) fn register_data_stuff(engine: &mut Engine) {
    engine
        .register_type_with_name::<DataMap>("DataMap")
        .register_indexer_get_set(rhai_get, rhai_set)
        .register_fn("get_or_new_inventory", |v: &mut DataMap, id: Id| v.inventory_mut(id).clone());

    engine
        .register_type_with_name::<Inventory>("Inventory")
        .register_fn("take", Inventory::take)
        .register_fn("add", Inventory::add)
        .register_indexer_get_set(Inventory::get, Inventory::insert);

    engine
        .register_type_with_name::<Id>("Id")
        .register_fn("==", |a: Id, b: Id| a == b)
        .register_fn("!=", |a: Id, b: Id| a != b)
        .register_type_with_name::<TileId>("TileId")
        .register_fn("==", |a: TileId, b: TileId| a == b)
        .register_fn("!=", |a: TileId, b: TileId| a != b)
        .register_type_with_name::<ModelId>("ModelId")
        .register_fn("==", |a: ModelId, b: ModelId| a == b)
        .register_fn("!=", |a: ModelId, b: ModelId| a != b)
        .register_fn("contains", |v: &mut HashSet<Id>, id: Id| -> bool { v.contains(&id) });

    engine
        .register_type_with_name::<ItemStack>("ItemStack")
        .register_fn("ItemStack", |id: Id, amount: ItemAmount| -> ItemStack { ItemStack { id, amount } })
        .register_fn("ItemStack", |amount: ItemAmount, id: Id| -> ItemStack { ItemStack { id, amount } })
        .register_get("id", |v: &mut ItemStack| -> Id { v.id })
        .register_get("amount", |v: &mut ItemStack| -> ItemAmount { v.amount });

    engine
        .register_type_with_name::<HashMap<TileCoord, Id>>("TileMap")
        .register_indexer_get(|v: &mut HashMap<TileCoord, Id>, coord: TileCoord| -> Dynamic {
            if let Some(v) = v.get(&coord).copied() {
                Dynamic::from(v)
            } else {
                Dynamic::UNIT
            }
        })
        .register_indexer_set(|v: &mut HashMap<TileCoord, Id>, coord: TileCoord, id: Id| {
            v.insert(coord, id);
        })
        .register_fn("contains", |v: &mut HashMap<TileCoord, Id>, coord: TileCoord| -> bool {
            v.contains_key(&coord)
        })
        .register_fn("keys", |v: &mut HashMap<TileCoord, Id>| -> Dynamic {
            Dynamic::from_iter(v.keys().cloned())
        })
        .register_fn("TileMap", HashMap::<TileCoord, Id>::new)
        .register_fn("TileMap", |v: Vec<(TileCoord, Id)>| -> HashMap<TileCoord, Id> {
            HashMap::<TileCoord, Id>::from_iter(v)
        });

    engine
        .register_type_with_name::<HashMap<Id, HashSet<Id>>>("MapSetId")
        .register_indexer_get(|v: &mut HashMap<Id, HashSet<Id>>, id: Id| -> Dynamic {
            if let Some(v) = v.get(&id).cloned() {
                Dynamic::from_iter(v)
            } else {
                Dynamic::UNIT
            }
        })
        .register_fn("keys", |v: HashMap<Id, HashSet<Id>>| -> Dynamic { Dynamic::from_iter(v.into_keys()) });

    engine
        .register_type_with_name::<ItemDef>("ItemDef")
        .register_get("id", |v: &mut ItemDef| -> Id { v.id })
        .register_fn("==", |a: ItemDef, b: ItemDef| a == b)
        .register_fn("!=", |a: ItemDef, b: ItemDef| a != b);
    engine
        .register_type_with_name::<RecipeDef>("RecipeDef")
        .register_get("inputs", |v: &mut RecipeDef| -> Dynamic {
            if let Some(v) = &v.inputs {
                Dynamic::from_iter(v.iter().cloned())
            } else {
                Dynamic::UNIT
            }
        })
        .register_get("outputs", |v: &mut RecipeDef| -> Dynamic {
            Dynamic::from_iter(v.outputs.iter().cloned())
        });
    engine.register_type_with_name::<TileDef>("TileDef");
    engine.register_type_with_name::<TagDef>("TagDef");

    engine.register_fn("as_recipe", |id: Id| {
        match resources::global::resource_man().registry.recipe_defs.get(&id).cloned() {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_tile", |id: Id| {
        match resources::global::resource_man().registry.tile_defs.get(&TileId(id)).cloned() {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_item", |id: Id| {
        match resources::global::resource_man().registry.item_defs.get(&id).cloned() {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_tag", |id: Id| {
        match resources::global::resource_man().registry.tag_defs.get(&id).cloned() {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
}
