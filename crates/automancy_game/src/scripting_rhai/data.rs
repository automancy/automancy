use core::any::TypeId;
use std::collections::BTreeMap;

use automancy_data::{
    game::{
        coord::{TileCoord, TileCoordBounds},
        generic::{DataMap, Datum},
        inventory::{Inventory, ItemAmount, ItemStack},
    },
    id::{Id, ItemId, ModelId, RecipeId, RenderId, TagId, TileId},
    id_map::{IdCoordMap, IdMap, IdSet},
    rendering::colors::Rgba,
};
use rhai::{Dynamic, Engine};

use crate::{
    resources,
    resources::types::{item::ItemDef, recipe::RecipeDef, tag::TagDef, tile::TileDef},
};

fn data_into_dynamic(v: Datum) -> Dynamic {
    match v {
        Datum::Id(v) => Dynamic::from(v),
        Datum::SetId(v) => Dynamic::from(v),
        Datum::MapSetId(v) => Dynamic::from(v),
        Datum::MapCoordId(v) => Dynamic::from(v),
        Datum::TileId(v) => Dynamic::from(v),
        Datum::SetTileId(v) => Dynamic::from(v),
        Datum::MapSetTileId(v) => Dynamic::from(v),
        Datum::MapCoordTileId(v) => Dynamic::from(v),
        Datum::ItemId(v) => Dynamic::from(v),
        Datum::SetItemId(v) => Dynamic::from(v),
        Datum::MapSetItemId(v) => Dynamic::from(v),
        Datum::MapCoordItemId(v) => Dynamic::from(v),
        Datum::RecipeId(v) => Dynamic::from(v),
        Datum::SetRecipeId(v) => Dynamic::from(v),
        Datum::MapSetRecipeId(v) => Dynamic::from(v),
        Datum::MapCoordRecipeId(v) => Dynamic::from(v),
        Datum::TagId(v) => Dynamic::from(v),
        Datum::SetTagId(v) => Dynamic::from(v),
        Datum::MapSetTagId(v) => Dynamic::from(v),
        Datum::MapCoordTagId(v) => Dynamic::from(v),
        Datum::CategoryId(v) => Dynamic::from(v),
        Datum::SetCategoryId(v) => Dynamic::from(v),
        Datum::MapSetCategoryId(v) => Dynamic::from(v),
        Datum::MapCoordCategoryId(v) => Dynamic::from(v),
        Datum::ResearchId(v) => Dynamic::from(v),
        Datum::SetResearchId(v) => Dynamic::from(v),
        Datum::MapSetResearchId(v) => Dynamic::from(v),
        Datum::MapCoordResearchId(v) => Dynamic::from(v),
        Datum::ScriptId(v) => Dynamic::from(v),
        Datum::SetScriptId(v) => Dynamic::from(v),
        Datum::MapSetScriptId(v) => Dynamic::from(v),
        Datum::MapCoordScriptId(v) => Dynamic::from(v),
        Datum::ModelId(v) => Dynamic::from(v),
        Datum::SetModelId(v) => Dynamic::from(v),
        Datum::MapSetModelId(v) => Dynamic::from(v),
        Datum::MapCoordModelId(v) => Dynamic::from(v),
        Datum::RenderId(v) => Dynamic::from(v),
        Datum::SetRenderId(v) => Dynamic::from(v),
        Datum::MapSetRenderId(v) => Dynamic::from(v),
        Datum::MapCoordRenderId(v) => Dynamic::from(v),

        Datum::TileCoord(v) => Dynamic::from(v),
        Datum::VecTileCoord(v) => Dynamic::from_iter(v),
        Datum::TileCoordBounds(v) => Dynamic::from(v),

        Datum::ItemStack(v) => Dynamic::from(v),
        Datum::Inventory(v) => Dynamic::from(v),

        Datum::Color(v) => Dynamic::from(v),
        Datum::VecColor(v) => Dynamic::from(v),

        Datum::Int(v) => Dynamic::from_int(v),
        Datum::VecInt(v) => Dynamic::from(v),
        Datum::UInt(v) => Dynamic::from(v),
        Datum::VecUInt(v) => Dynamic::from(v),
        Datum::Float(v) => Dynamic::from(v),
        Datum::VecFloat(v) => Dynamic::from(v),
        Datum::Bool(v) => Dynamic::from_bool(v),
    }
}

fn data_from_dynamic(v: Dynamic) -> Option<Datum> {
    let id = v.type_id();

    macro_rules! impl_conv {
        (
            $variant:expr => $ty:ty
            $(, $variants:expr => $tys:ty)*
        ) => {
            if id == TypeId::of::<$ty>() {
                Some($variant(v.cast::<$ty>()))
            }
            $(
                else if id == TypeId::of::<$tys>() {
                    Some($variants(v.cast::<$tys>()))
                }
            )*
            else {
                None
            }
        };
    }

    impl_conv!(
        Datum::Id => Id,
        Datum::SetId => IdSet<Id>,
        Datum::MapSetId => IdMap<Id, IdSet<Id>>,

        Datum::MapCoordId => IdCoordMap<Id>,
        Datum::MapCoordModelId => IdCoordMap<ModelId>,

        Datum::TileCoord => TileCoord,
        Datum::VecTileCoord => Vec<TileCoord>,
        Datum::TileCoordBounds => TileCoordBounds,

        Datum::Int => ItemAmount,
        Datum::Bool => bool,
        Datum::Color => Rgba,
        Datum::Inventory => Inventory
    )
}

fn rhai_get(data: &mut DataMap, id: Id) -> Dynamic {
    if let Some(datum) = data.get(id) {
        data_into_dynamic(datum)
    } else {
        Dynamic::UNIT
    }
}

fn rhai_set(data: &mut DataMap, id: Id, dynamic: Dynamic) {
    if let Some(datum) = data_from_dynamic(dynamic) {
        data.set(id, datum);
    }
}

pub(crate) fn register_data_stuff(engine: &mut Engine) {
    engine
        .register_type_with_name::<DataMap>("DataMap")
        .register_indexer_get_set(rhai_get, rhai_set)
        .register_fn("get_or_new_inventory", |v: &mut DataMap, id: Id| {
            v.inventory_mut_or_default(id).clone()
        });

    engine
        .register_type_with_name::<Inventory>("Inventory")
        .register_fn("take", Inventory::take)
        .register_fn("add", Inventory::add)
        .register_indexer_get_set(|v: &mut Inventory, id: ItemId| -> ItemAmount { v.get(id) }, Inventory::set);

    engine
        .register_type_with_name::<Id>("Id")
        .register_fn("==", |a: Id, b: Id| a == b)
        .register_fn("!=", |a: Id, b: Id| a != b)
        .register_type_with_name::<IdSet<Id>>("IdSet")
        .register_fn("IdSet", IdSet::<Id>::new)
        .register_type_with_name::<TileId>("TileId")
        .register_fn("TileId", |v: Id| TileId(v))
        .register_fn("==", |a: TileId, b: TileId| a == b)
        .register_fn("!=", |a: TileId, b: TileId| a != b)
        .register_type_with_name::<IdSet<TileId>>("TileIdSet")
        .register_fn("TileIdSet", IdSet::<TileId>::new)
        .register_type_with_name::<ItemId>("ItemId")
        .register_fn("ItemId", |v: Id| ItemId(v))
        .register_fn("==", |a: ItemId, b: ItemId| a == b)
        .register_fn("!=", |a: ItemId, b: ItemId| a != b)
        .register_type_with_name::<IdSet<ItemId>>("ItemIdSet")
        .register_fn("ItemIdSet", IdSet::<ItemId>::new)
        .register_type_with_name::<ModelId>("ModelId")
        .register_fn("ModelId", |v: Id| ModelId(v))
        .register_fn("==", |a: ModelId, b: ModelId| a == b)
        .register_fn("!=", |a: ModelId, b: ModelId| a != b)
        .register_type_with_name::<IdSet<ModelId>>("ModelIdSet")
        .register_fn("ModelIdSet", IdSet::<ModelId>::new)
        .register_type_with_name::<RecipeId>("RecipeId")
        .register_fn("RecipeId", |v: Id| RecipeId(v))
        .register_fn("==", |a: RecipeId, b: RecipeId| a == b)
        .register_fn("!=", |a: RecipeId, b: RecipeId| a != b)
        .register_type_with_name::<IdSet<RecipeId>>("RecipeIdSet")
        .register_fn("RecipeIdSet", IdSet::<RecipeId>::new)
        .register_type_with_name::<TagId>("TagId")
        .register_fn("TagId", |v: Id| TagId(v))
        .register_fn("==", |a: TagId, b: TagId| a == b)
        .register_fn("!=", |a: TagId, b: TagId| a != b)
        .register_type_with_name::<IdSet<TagId>>("TagIdSet")
        .register_fn("TagIdSet", IdSet::<TagId>::new)
        .register_type_with_name::<RenderId>("RenderId")
        .register_fn("RenderId", |v: Id| RenderId(v))
        .register_fn("==", |a: RenderId, b: RenderId| a == b)
        .register_fn("!=", |a: RenderId, b: RenderId| a != b)
        .register_type_with_name::<IdSet<RenderId>>("RenderIdSet")
        .register_fn("RenderIdSet", IdSet::<RenderId>::new)
        .register_fn("contains", |v: &mut IdSet<Id>, id: Id| -> bool { v.contains(&id) });

    engine
        .register_type_with_name::<ItemStack>("ItemStack")
        .register_fn("ItemStack", |id: ItemId, amount: ItemAmount| -> ItemStack {
            ItemStack {
                id,
                amount,
            }
        })
        .register_fn("ItemStack", |amount: ItemAmount, id: ItemId| -> ItemStack {
            ItemStack {
                id,
                amount,
            }
        })
        .register_get("id", |v: &mut ItemStack| -> ItemId { v.id })
        .register_get("amount", |v: &mut ItemStack| -> ItemAmount { v.amount });

    engine
        .register_type_with_name::<BTreeMap<TileCoord, Id>>("MapCoordId")
        .register_indexer_get(|v: &mut BTreeMap<TileCoord, Id>, coord: TileCoord| -> Dynamic {
            if let Some(v) = v.get(&coord).copied() {
                Dynamic::from(v)
            } else {
                Dynamic::UNIT
            }
        })
        .register_indexer_set(|v: &mut BTreeMap<TileCoord, Id>, coord: TileCoord, id: Id| {
            v.insert(coord, id);
        })
        .register_fn("contains", |v: &mut BTreeMap<TileCoord, Id>, coord: TileCoord| -> bool {
            v.contains_key(&coord)
        })
        .register_fn("keys", |v: &mut BTreeMap<TileCoord, Id>| -> Dynamic {
            Dynamic::from_iter(v.keys().cloned())
        })
        .register_fn("MapCoordId", BTreeMap::<TileCoord, Id>::new)
        .register_fn("MapCoordId", |v: Vec<(TileCoord, Id)>| -> BTreeMap<TileCoord, Id> {
            BTreeMap::from_iter(v)
        });

    engine
        .register_type_with_name::<BTreeMap<TileCoord, ModelId>>("MapCoordModelId")
        .register_indexer_get(|v: &mut BTreeMap<TileCoord, ModelId>, coord: TileCoord| -> Dynamic {
            if let Some(v) = v.get(&coord).copied() {
                Dynamic::from(v)
            } else {
                Dynamic::UNIT
            }
        })
        .register_indexer_set(|v: &mut BTreeMap<TileCoord, ModelId>, coord: TileCoord, id: ModelId| {
            v.insert(coord, id);
        })
        .register_fn("contains", |v: &mut BTreeMap<TileCoord, ModelId>, coord: TileCoord| -> bool {
            v.contains_key(&coord)
        })
        .register_fn("keys", |v: &mut BTreeMap<TileCoord, ModelId>| -> Dynamic {
            Dynamic::from_iter(v.keys().cloned())
        })
        .register_fn("MapCoordModelId", BTreeMap::<TileCoord, ModelId>::new)
        .register_fn("MapCoordModelId", |v: Vec<(TileCoord, ModelId)>| -> BTreeMap<TileCoord, ModelId> {
            BTreeMap::from_iter(v)
        });

    engine
        .register_type_with_name::<IdMap<Id, IdSet<Id>>>("MapSetId")
        .register_indexer_get(|v: &mut IdMap<Id, IdSet<Id>>, id: Id| -> Dynamic {
            if let Some(v) = v.get(&id).cloned() {
                Dynamic::from_iter(v)
            } else {
                Dynamic::UNIT
            }
        })
        .register_fn("keys", |v: IdMap<Id, IdSet<Id>>| -> Dynamic { Dynamic::from_iter(v.into_keys()) })
        .register_type_with_name::<IdMap<ModelId, IdSet<ModelId>>>("MapSetModelId")
        .register_indexer_get(|v: &mut IdMap<ModelId, IdSet<ModelId>>, id: ModelId| -> Dynamic {
            if let Some(v) = v.get(&id).cloned() {
                Dynamic::from_iter(v)
            } else {
                Dynamic::UNIT
            }
        })
        .register_fn("keys", |v: IdMap<ModelId, IdSet<ModelId>>| -> Dynamic {
            Dynamic::from_iter(v.into_keys())
        });

    engine
        .register_type_with_name::<ItemDef>("ItemDef")
        .register_get("id", |v: &mut ItemDef| -> Id { *v.id })
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
        match resources::global::resource_man().registry.recipe_defs.get(&RecipeId(id)).cloned() {
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
        match resources::global::resource_man().registry.item_defs.get(&ItemId(id)).cloned() {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
    engine.register_fn("as_tag", |id: Id| {
        match resources::global::resource_man().registry.tag_defs.get(&TagId(id)).cloned() {
            Some(v) => Dynamic::from(v),
            None => Dynamic::UNIT,
        }
    });
}
