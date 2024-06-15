use std::any::TypeId;
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use hashbrown::{HashMap, HashSet};
use rhai::Dynamic;
use serde::{Deserialize, Serialize};

use automancy_defs::coord::{TileBounds, TileCoord, TileUnit};
use automancy_defs::glam::IVec2;
use automancy_defs::hex;
use automancy_defs::hexx::{Hex, OffsetHexMode};
use automancy_defs::id::{Id, IdRaw, Interner};
use yakui::Color;

use crate::data::inventory::{Inventory, InventoryRaw};
use crate::data::stack::ItemAmount;

pub mod inventory;
pub mod item;
pub mod stack;

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

    pub fn rhai_value(self) -> Dynamic {
        match self {
            Data::Inventory(v) => Dynamic::from(v),
            Data::Coord(v) => Dynamic::from(v),
            Data::VecCoord(v) => Dynamic::from_iter(v),
            Data::Id(v) => Dynamic::from_int(v.into()),
            Data::VecId(v) => Dynamic::from(v),
            Data::SetId(v) => Dynamic::from(v),
            Data::Amount(v) => Dynamic::from_int(v),
            Data::Bool(v) => Dynamic::from_bool(v),
            Data::Color(v) => Dynamic::from(v),
            Data::TileBounds(v) => Dynamic::from(v),
            Data::TileMap(v) => Dynamic::from(v),
            Data::MapSetId(v) => Dynamic::from(v),
        }
    }

    pub fn from_rhai(v: Dynamic) -> Option<Self> {
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

impl TryFrom<Dynamic> for Data {
    type Error = ();

    fn try_from(value: Dynamic) -> Result<Self, Self::Error> {
        Data::from_rhai(value).ok_or(())
    }
}

impl Data {
    fn try_to_raw(&self, interner: &Interner) -> Option<DataRaw> {
        Some(match self {
            Data::Inventory(v) => DataRaw::Inventory(v.to_raw(interner)),
            Data::Coord(v) => DataRaw::Coord(*v),
            Data::VecCoord(v) => DataRaw::VecCoord(v.clone()),
            Data::Id(v) => DataRaw::Id(interner.resolve(*v).map(IdRaw::parse)?),
            Data::VecId(v) => DataRaw::VecId(
                v.iter()
                    .flat_map(|id| interner.resolve(*id).map(IdRaw::parse))
                    .collect(),
            ),
            Data::SetId(v) => DataRaw::SetId(
                v.iter()
                    .flat_map(|id| interner.resolve(*id).map(IdRaw::parse))
                    .collect(),
            ),
            Data::Amount(v) => DataRaw::Amount(*v),
            Data::Bool(v) => DataRaw::Bool(*v),
            Data::Color(v) => DataRaw::Color(hex::encode([v.r, v.g, v.b, v.a])),
            Data::TileBounds(v) => DataRaw::TileBounds(*v),
            Data::TileMap(v) => DataRaw::TileMap(
                v.iter()
                    .flat_map(|(coord, id)| {
                        Some(*coord).zip(interner.resolve(*id).map(IdRaw::parse))
                    })
                    .collect(),
            ),
            Data::MapSetId(v) => DataRaw::MapSetId(
                v.iter()
                    .flat_map(|(id, set)| {
                        interner.resolve(*id).map(IdRaw::parse).zip(Some(
                            set.iter()
                                .flat_map(|id| interner.resolve(*id).map(IdRaw::parse))
                                .collect(),
                        ))
                    })
                    .collect(),
            ),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct DataMap(BTreeMap<Id, Data>);

impl From<DataMap> for BTreeMap<Id, Data> {
    fn from(value: DataMap) -> Self {
        value.0
    }
}

impl From<BTreeMap<Id, Data>> for DataMap {
    fn from(value: BTreeMap<Id, Data>) -> Self {
        Self(value)
    }
}

impl Deref for DataMap {
    type Target = BTreeMap<Id, Data>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DataMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DataMap {
    pub fn into_inner(self) -> BTreeMap<Id, Data> {
        self.0
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataRaw {
    Inventory(InventoryRaw),
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    Id(IdRaw),
    Color(String),
    VecId(Vec<IdRaw>),
    SetId(Vec<IdRaw>),
    Amount(ItemAmount),
    Bool(bool),
    TileBounds(TileBounds),
    VecOffsetCoord(Vec<IVec2>),
    TileMap(Vec<(TileCoord, IdRaw)>),
    TileMapOffsetCoord(Vec<(IVec2, IdRaw)>),
    MapSetId(Vec<(IdRaw, Vec<IdRaw>)>),
}

impl DataRaw {
    pub fn try_to_data(&self, interner: &Interner) -> Option<Data> {
        Some(match self {
            DataRaw::Inventory(v) => Data::Inventory(v.to_inventory(interner)),
            DataRaw::Coord(v) => Data::Coord(*v),
            DataRaw::VecCoord(v) => Data::VecCoord(v.clone()),
            DataRaw::Id(v) => Data::Id(interner.get(v.to_string())?),
            DataRaw::VecId(v) => Data::VecId(
                v.iter()
                    .flat_map(|id| interner.get(id.to_string()))
                    .collect(),
            ),
            DataRaw::SetId(v) => Data::SetId(
                v.iter()
                    .flat_map(|id| interner.get(id.to_string()))
                    .collect(),
            ),
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
            DataRaw::VecOffsetCoord(v) => {
                Data::VecCoord(v.iter().map(|v| offset_to_tile(v.to_array())).collect())
            }
            DataRaw::TileBounds(v) => Data::TileBounds(*v),
            DataRaw::TileMap(v) => {
                Data::TileMap(HashMap::from_iter(v.iter().flat_map(|(coord, id)| {
                    Some(*coord).zip(interner.get(id.to_string()))
                })))
            }
            DataRaw::TileMapOffsetCoord(v) => {
                Data::TileMap(HashMap::from_iter(v.iter().flat_map(|(coord, id)| {
                    Some(offset_to_tile(coord.to_array())).zip(interner.get(id.to_string()))
                })))
            }
            DataRaw::MapSetId(v) => Data::MapSetId(
                v.iter()
                    .flat_map(|(id, set)| {
                        interner.get(id.to_string()).zip(Some(
                            set.iter()
                                .flat_map(|id| interner.get(id.to_string()))
                                .collect(),
                        ))
                    })
                    .collect(),
            ),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataMapRaw(HashMap<String, DataRaw>);

impl From<DataMapRaw> for HashMap<String, DataRaw> {
    fn from(value: DataMapRaw) -> Self {
        value.0
    }
}

impl From<HashMap<String, DataRaw>> for DataMapRaw {
    fn from(value: HashMap<String, DataRaw>) -> Self {
        Self(value)
    }
}

impl DataMapRaw {
    pub fn into_inner(self) -> HashMap<String, DataRaw> {
        self.0
    }

    pub fn intern_to_data(&self, interner: &mut Interner) -> DataMap {
        DataMap(
            self.0
                .iter()
                .flat_map(|(key, value)| {
                    match value {
                        DataRaw::Id(v) => Some(Data::Id(interner.get_or_intern(v.to_string()))),
                        DataRaw::VecId(v) => Some(Data::VecId(
                            v.iter()
                                .map(|id| interner.get_or_intern(id.to_string()))
                                .collect(),
                        )),
                        DataRaw::SetId(v) => Some(Data::SetId(
                            v.iter()
                                .map(|id| interner.get_or_intern(id.to_string()))
                                .collect(),
                        )),
                        DataRaw::TileMap(v) => Some(Data::TileMap(
                            v.iter()
                                .map(|(coord, id)| (*coord, interner.get_or_intern(id.to_string())))
                                .collect(),
                        )),
                        DataRaw::TileMapOffsetCoord(v) => Some(Data::TileMap(
                            v.iter()
                                .map(|(coord, id)| {
                                    (
                                        offset_to_tile(coord.to_array()),
                                        interner.get_or_intern(id.to_string()),
                                    )
                                })
                                .collect(),
                        )),
                        DataRaw::MapSetId(v) => Some(Data::MapSetId(
                            v.iter()
                                .map(|(id, set)| {
                                    (
                                        interner.get_or_intern(id.to_string()),
                                        set.iter()
                                            .map(|id| interner.get_or_intern(id.to_string()))
                                            .collect(),
                                    )
                                })
                                .collect(),
                        )),
                        rest => rest.try_to_data(interner),
                    }
                    .map(|v| (IdRaw::parse(key).to_id(interner), v))
                })
                .collect(),
        )
    }

    pub fn to_data(&self, interner: &Interner) -> DataMap {
        DataMap(
            self.0
                .iter()
                .flat_map(|(key, value)| {
                    interner
                        .get(IdRaw::parse(key).to_string())
                        .and_then(|key| value.try_to_data(interner).map(|v| (key, v)))
                })
                .collect(),
        )
    }
}
