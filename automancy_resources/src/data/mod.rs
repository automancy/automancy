use std::any::TypeId;
use std::collections::{BTreeMap, HashSet};
use std::ops::{Deref, DerefMut};

use egui::Rgba;
use rhai::Dynamic;
use serde::{Deserialize, Serialize};

use automancy_defs::coord::TileCoord;
use automancy_defs::id::{Id, IdRaw, Interner};
use hashbrown::HashMap;

use crate::data::inventory::{Inventory, InventoryRaw};
use crate::data::stack::ItemAmount;

pub mod inventory;
pub mod item;
pub mod stack;

/// Represents the data a tile entity holds. This data is given to functions.
#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    Inventory(Inventory),
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    Id(Id),
    Color(Rgba),
    VecId(Vec<Id>),
    SetId(HashSet<Id>),
    Amount(ItemAmount),
    Bool(bool),
}

impl Data {
    pub fn into_inventory(self) -> Option<Inventory> {
        if let Self::Inventory(v) = self {
            return Some(v);
        }
        None
    }

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

    pub fn into_vec_coord(self) -> Option<Vec<TileCoord>> {
        if let Self::VecCoord(v) = self {
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

    pub fn into_vec_id(self) -> Option<Vec<Id>> {
        if let Self::VecId(v) = self {
            return Some(v);
        }
        None
    }

    pub fn into_set_id(self) -> Option<HashSet<Id>> {
        if let Self::SetId(v) = self {
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
            Data::VecId(v) => Dynamic::from_iter(v),
            Data::SetId(v) => Dynamic::from(v),
            Data::Amount(v) => Dynamic::from_int(v),
            Data::Bool(v) => Dynamic::from_bool(v),
            Data::Color(v) => Dynamic::from(v),
        }
    }

    pub fn from_rhai(v: Dynamic) -> Option<Self> {
        Some(if v.type_id() == TypeId::of::<TileCoord>() {
            Data::Coord(v.cast())
        } else if v.type_id() == TypeId::of::<Id>() {
            Data::Id(v.cast())
        } else if v.type_id() == TypeId::of::<ItemAmount>() {
            Data::Amount(v.cast())
        } else if v.type_id() == TypeId::of::<bool>() {
            Data::Bool(v.cast())
        } else if v.type_id() == TypeId::of::<Inventory>() {
            Data::Inventory(v.cast())
        } else if v.type_id() == TypeId::of::<Vec<TileCoord>>() {
            Data::VecCoord(v.cast())
        } else if v.type_id() == TypeId::of::<Vec<Id>>() {
            Data::VecId(v.cast())
        } else if v.type_id() == TypeId::of::<HashSet<Id>>() {
            Data::SetId(v.cast())
        } else {
            return None;
        })
    }
}

impl TryFrom<Dynamic> for Data {
    type Error = ();

    fn try_from(value: Dynamic) -> Result<Self, Self::Error> {
        let ty = value.type_id();

        if ty == TypeId::of::<Inventory>() {
            Ok(Data::Inventory(value.cast()))
        } else if ty == TypeId::of::<TileCoord>() {
            Ok(Data::Coord(value.cast()))
        } else if ty == TypeId::of::<Vec<TileCoord>>() {
            Ok(Data::VecCoord(value.cast()))
        } else if ty == TypeId::of::<Id>() {
            Ok(Data::Id(value.cast()))
        } else if ty == TypeId::of::<Vec<Id>>() {
            Ok(Data::VecId(value.cast()))
        } else if ty == TypeId::of::<ItemAmount>() {
            Ok(Data::Amount(value.cast()))
        } else if ty == TypeId::of::<bool>() {
            Ok(Data::Bool(value.cast()))
        } else {
            Err(())
        }
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
            Data::Color(v) => DataRaw::Color(hex::encode(v.to_srgba_unmultiplied())),
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

#[derive(Debug, Serialize, Deserialize)]
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
            DataRaw::SetId(v) => Data::VecId(
                v.iter()
                    .flat_map(|id| interner.get(id.to_string()))
                    .collect(),
            ),
            DataRaw::Amount(v) => Data::Amount(*v),
            DataRaw::Bool(v) => Data::Bool(*v),
            DataRaw::Color(v) => {
                let mut color = hex::decode(v).ok()?.into_iter();
                Data::Color(Rgba::from_srgba_premultiplied(
                    color.next()?,
                    color.next()?,
                    color.next()?,
                    color.next().unwrap_or(255),
                ))
            }
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
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
