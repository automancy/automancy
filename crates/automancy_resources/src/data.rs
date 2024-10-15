use crate::inventory::{Inventory, InventoryRaw};
use automancy_defs::{
    coord::{TileBounds, TileCoord, TileUnit},
    resolve_map_id_of, resolve_map_v_id,
    stack::{ItemAmount, ItemStack},
    try_parse_ids,
};
use automancy_defs::{glam::IVec2, try_parse_map_id_of};
use automancy_defs::{hex, try_parse_map_v_id};
use automancy_defs::{
    hexx::{Hex, OffsetHexMode},
    parse_ids, parse_map_id_of, parse_map_v_id,
};
use automancy_defs::{
    id::{Id, Interner},
    resolve_ids,
};
use hashbrown::{HashMap, HashSet};
use rhai::Dynamic;
use serde::{Deserialize, Serialize};
use std::{
    any::TypeId,
    collections::{
        btree_map::{self, Entry},
        BTreeMap,
    },
};
use yakui::Color;

fn offset_to_tile(a: [TileUnit; 2]) -> TileCoord {
    TileCoord::from(Hex::from_offset_coordinates(a, OffsetHexMode::EvenRows))
}

/// Represents the data a tile entity holds. This data is given to functions.
#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    Inventory(Inventory),
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    TileBounds(TileBounds),
    Id(Id),
    Color(Color),
    VecId(Vec<Id>),
    SetId(HashSet<Id>),
    Amount(ItemAmount),
    Bool(bool),
    TileMap(HashMap<TileCoord, Id>),
    MapSetId(HashMap<Id, HashSet<Id>>),
}

impl Data {
    pub fn into_coord(self) -> Option<TileCoord> {
        if let Self::Coord(v) = self {
            return Some(v);
        }
        None
    }

    pub fn into_bool(self) -> Option<bool> {
        if let Self::Bool(v) = self {
            return Some(v);
        }
        None
    }

    pub fn into_id(self) -> Option<Id> {
        if let Self::Id(v) = self {
            return Some(v);
        }
        None
    }

    pub fn into_amount(self) -> Option<ItemAmount> {
        if let Self::Amount(v) = self {
            return Some(v);
        }
        None
    }

    pub fn into_dynamic(self) -> Dynamic {
        match self {
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

    pub fn from_dynamic(v: Dynamic) -> Option<Self> {
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
}

impl Data {
    fn try_to_raw(&self, interner: &Interner) -> Option<DataRaw> {
        Some(match self {
            Data::Id(v) => DataRaw::Id(interner.resolve(*v)?.to_string()),
            Data::VecId(v) => DataRaw::VecId(resolve_ids(v.iter().cloned(), interner)),
            Data::SetId(v) => DataRaw::SetId(resolve_ids(v.iter().cloned(), interner)),
            Data::Amount(v) => DataRaw::Amount(*v),
            Data::Bool(v) => DataRaw::Bool(*v),
            Data::Color(v) => DataRaw::Color(hex::encode([v.r, v.g, v.b, v.a])),
            Data::TileBounds(v) => DataRaw::TileBounds(*v),
            Data::TileMap(v) => {
                DataRaw::TileMap(resolve_map_v_id(v.iter().map(|(a, b)| (*a, *b)), interner))
            }
            Data::Inventory(v) => DataRaw::Inventory(v.to_raw(interner)),
            Data::Coord(v) => DataRaw::Coord(*v),
            Data::VecCoord(v) => DataRaw::VecCoord(v.clone()),
            Data::MapSetId(v) => DataRaw::MapSetId(resolve_map_id_of(
                v.iter()
                    .map(|(id, set)| (*id, resolve_ids(set.iter().cloned(), interner))),
                interner,
            )),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataMap(BTreeMap<Id, Data>);

impl IntoIterator for DataMap {
    type Item = (Id, Data);

    type IntoIter = btree_map::IntoIter<Id, Data>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl DataMap {
    pub fn keys(&self) -> impl Iterator<Item = &Id> {
        self.0.keys()
    }

    pub fn rhai_get(&mut self, id: Id) -> Dynamic {
        if let Some(v) = self.get(id).cloned() {
            v.into_dynamic()
        } else {
            Dynamic::UNIT
        }
    }

    pub fn rhai_set(&mut self, id: Id, data: Dynamic) {
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

    pub fn get(&self, id: Id) -> Option<&Data> {
        self.0.get(&id)
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut Data> {
        self.0.get_mut(&id)
    }

    pub fn set(&mut self, id: Id, data: Data) {
        self.0.insert(id, data);
    }

    pub fn remove(&mut self, id: Id) -> Option<Data> {
        self.0.remove(&id)
    }

    pub fn entry(&mut self, id: Id) -> Entry<'_, Id, Data> {
        self.0.entry(id)
    }

    pub fn to_raw(&self, interner: &Interner) -> DataMapRaw {
        DataMapRaw(
            self.0
                .iter()
                .flat_map(|(key, value)| {
                    interner
                        .resolve(*key)
                        .and_then(|key| value.try_to_raw(interner).map(|v| (key.to_string(), v)))
                })
                .collect(),
        )
    }

    pub fn contains_id(&self, key: Id, id: Id) -> bool {
        if let Some(v) = self.get(key) {
            match v {
                Data::Inventory(v) => v.contains_key(&id),
                Data::Id(v) => *v == id,
                Data::VecId(v) => v.contains(&id),
                Data::SetId(v) => v.contains(&id),
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn contains_stack(&mut self, key: Id, stack: ItemStack) -> bool {
        if let Some(Data::Inventory(v)) = self.get_mut(key) {
            v.contains(stack)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataRaw {
    Inventory(InventoryRaw),
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    Id(String),
    Color(String),
    VecId(Vec<String>),
    SetId(Vec<String>),
    Amount(ItemAmount),
    Bool(bool),
    TileBounds(TileBounds),
    VecOffsetCoord(Vec<IVec2>),
    TileMap(Vec<(TileCoord, String)>),
    TileMapOffsetCoord(Vec<(IVec2, String)>),
    MapSetId(Vec<(String, Vec<String>)>),
}

impl DataRaw {
    pub fn to_data(&self, interner: &Interner) -> Option<Data> {
        Some(match self {
            DataRaw::Id(v) => Data::Id(Id::try_parse(v, interner)?),
            DataRaw::VecId(v) => Data::VecId(try_parse_ids(v.iter().cloned(), interner)),
            DataRaw::SetId(v) => Data::SetId(try_parse_ids(v.iter().cloned(), interner)),
            DataRaw::TileMap(v) => Data::TileMap(try_parse_map_v_id(
                v.iter()
                    .map(|(coord, id)| (offset_to_tile(coord.to_array()), id)),
                interner,
            )),
            DataRaw::TileMapOffsetCoord(v) => Data::TileMap(try_parse_map_v_id(
                v.iter()
                    .map(|(coord, id)| (offset_to_tile(coord.to_array()), id)),
                interner,
            )),
            DataRaw::MapSetId(v) => Data::MapSetId(try_parse_map_id_of(
                v.iter()
                    .map(|(id, set)| (id, try_parse_ids(set.iter().cloned(), interner))),
                interner,
            )),
            DataRaw::Inventory(v) => Data::Inventory(v.try_to_inventory(interner)),
            DataRaw::Amount(v) => Data::Amount(*v),
            DataRaw::Bool(v) => Data::Bool(*v),
            DataRaw::Color(v) => {
                let mut color = hex::decode(v).ok()?.into_iter();
                Data::Color(Color {
                    r: color.next()?,
                    g: color.next()?,
                    b: color.next()?,
                    a: color.next().unwrap_or(255),
                })
            }
            DataRaw::Coord(v) => Data::Coord(*v),
            DataRaw::VecCoord(v) => Data::VecCoord(v.clone()),
            DataRaw::VecOffsetCoord(v) => {
                Data::VecCoord(v.iter().map(|v| offset_to_tile(v.to_array())).collect())
            }
            DataRaw::TileBounds(v) => Data::TileBounds(*v),
        })
    }

    pub fn intern_to_data(&self, interner: &mut Interner, namespace: Option<&str>) -> Option<Data> {
        Some(match self {
            DataRaw::Id(v) => Data::Id(Id::parse(v, interner, namespace)?),
            DataRaw::VecId(v) => Data::VecId(parse_ids(v.iter().cloned(), interner, namespace)),
            DataRaw::SetId(v) => Data::SetId(parse_ids(v.iter().cloned(), interner, namespace)),
            DataRaw::TileMap(v) => {
                Data::TileMap(parse_map_v_id(v.iter().cloned(), interner, namespace))
            }
            DataRaw::TileMapOffsetCoord(v) => Data::TileMap(parse_map_v_id(
                v.iter()
                    .map(|(coord, id)| (offset_to_tile(coord.to_array()), id)),
                interner,
                namespace,
            )),
            DataRaw::MapSetId(v) => Data::MapSetId(parse_map_id_of(
                v.iter()
                    .map(|(id, set)| (id, parse_ids(set.iter().cloned(), interner, namespace)))
                    .collect::<Vec<_>>()
                    .into_iter(),
                interner,
                namespace,
            )),
            DataRaw::Inventory(v) => Data::Inventory(v.to_inventory(interner, namespace)),
            rest => rest.to_data(interner)?,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataMapRaw(BTreeMap<String, DataRaw>);

impl From<DataMapRaw> for BTreeMap<String, DataRaw> {
    fn from(value: DataMapRaw) -> Self {
        value.0
    }
}

impl From<BTreeMap<String, DataRaw>> for DataMapRaw {
    fn from(value: BTreeMap<String, DataRaw>) -> Self {
        Self(value)
    }
}

impl DataMapRaw {
    pub fn into_inner(self) -> BTreeMap<String, DataRaw> {
        self.0
    }

    pub fn to_data(&self, interner: &Interner) -> DataMap {
        DataMap(
            self.0
                .iter()
                .flat_map(|(id, value)| {
                    Id::try_parse(id, interner)
                        .and_then(|key| value.to_data(interner).map(|v| (key, v)))
                })
                .collect(),
        )
    }

    pub fn intern_to_data(&self, interner: &mut Interner, namespace: Option<&str>) -> DataMap {
        DataMap(
            self.0
                .iter()
                .flat_map(|(key, value)| {
                    Id::parse(key, interner, namespace)
                        .zip(value.intern_to_data(interner, namespace))
                })
                .collect(),
        )
    }
}
