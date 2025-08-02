use automancy_data::{
    coord::TileCoord,
    game::{
        coord::{TileBounds, TileCoord},
        generic::{Data, DataMap},
        inventory::Inventory,
        item::ItemAmount,
    },
    id::{Id, ModelId, TileId},
    stack::{ItemAmount, ItemStack},
};
use hashbrown::{HashMap, HashSet};
use rhai::{Dynamic, Engine};

use crate::{
    generic::DataMap,
    inventory::Inventory,
    types::{
        item::ItemDef,
        script::{InstructionsDef, ScriptDef},
        tag::TagDef,
        tile::TileDef,
    },
};

pub fn data_into_dynamic(v: Data) -> Dynamic {
    match v {
        Data::Inventory(v) => Dynamic::from(v),
        Data::Coord(v) => Dynamic::from(v),
        Data::VecCoord(v) => Dynamic::from_iter(v),
        Data::TileBounds(v) => Dynamic::from(v),
        Data::Id(v) => Dynamic::from(v),
        Data::Color(v) => Dynamic::from(v),
        Data::VecId(v) => Dynamic::from_iter(v),
        Data::SetId(v) => Dynamic::from_iter(v),
        Data::Amount(v) => Dynamic::from_int(v),
        Data::Bool(v) => Dynamic::from_bool(v),
        Data::TileMap(v) => Dynamic::from(v),
        Data::MapSetId(v) => Dynamic::from(v),
    }
}

pub fn data_from_dynamic(v: Dynamic) -> Option<Data> {
    let id = v.type_id();

    Some(if id == TypeId::of::<TileCoord>() {
        Data::Coord(v.cast())
    } else if id == TypeId::of::<Id>() {
        Data::Id(v.cast())
    } else if id == TypeId::of::<ItemAmount>() {
        Data::Amount(v.cast())
    } else if id == TypeId::of::<bool>() {
        Data::Bool(v.cast())
    } else if id == TypeId::of::<Inventory>() {
        Data::Inventory(v.cast())
    } else if id == TypeId::of::<Vec<TileCoord>>() {
        Data::VecCoord(v.cast())
    } else if id == TypeId::of::<Vec<Id>>() {
        Data::VecId(v.cast())
    } else if id == TypeId::of::<HashSet<Id>>() {
        Data::SetId(v.cast())
    } else if id == TypeId::of::<TileBounds>() {
        Data::TileBounds(v.cast())
    } else if id == TypeId::of::<HashMap<TileCoord, Id>>() {
        Data::TileMap(v.cast())
    } else if id == TypeId::of::<HashMap<Id, HashSet<Id>>>() {
        Data::MapSetId(v.cast())
    } else {
        return None;
    })
}

pub fn rhai_get(dmap: &mut DataMap, id: Id) -> Dynamic {
    if let Some(v) = dmap.get(id).cloned() {
        v.into_dynamic()
    } else {
        Dynamic::UNIT
    }
}

pub fn rhai_set(dmap: &mut DataMap, id: Id, data: Dynamic) {
    if let Some(data) = Data::from_dynamic(data) {
        self.set(id, data);
    }
}

pub fn get_or_new_inventory(&mut self, id: Id) -> Dynamic {
    self.0
        .entry(id)
        .or_insert_with(|| Data::Inventory(Default::default()))
        .clone()
        .into_dynamic()
}

pub(crate) fn register_data_stuff(engine: &mut Engine) {
    engine
        .register_type_with_name::<DataMap>("DataMap")
        .register_indexer_get_set(rhai_get, rhai_set)
        .register_fn("get_or_new_inventory", get_or_new_inventory);

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

    engine.register_fn("as_script", |id: Id| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .registry
            .scripts
            .get(&id)
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_tile", |id: Id| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .registry
            .tiles
            .get(&TileId(id))
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_item", |id: Id| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .registry
            .items
            .get(&id)
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_tag", |id: Id| {
        match RESOURCE_MAN
            .read()
            .unwrap()
            .clone()
            .unwrap()
            .registry
            .tags
            .get(&id)
            .cloned()
        {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
}
