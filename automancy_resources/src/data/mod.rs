use std::any::TypeId;
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use rhai::{Dynamic, ImmutableString};
use serde::{Deserialize, Serialize};

use automancy_defs::coord::TileCoord;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::{Id, IdRaw, Interner};

use crate::data::inventory::{Inventory, InventoryRaw};
use crate::data::stack::ItemAmount;
use crate::ResourceManager;

pub mod inventory;
pub mod item;
pub mod stack;

/// Represents the data a tile entity holds. This data is given to functions.
#[derive(Debug, Clone)]
pub enum Data {
    Inventory(Inventory),
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    Id(Id),
    VecId(Vec<Id>),
    Amount(ItemAmount),
    Bool(bool),
}

impl Data {
    /// Gets the default Inventory.
    pub fn new_inventory() -> Self {
        Self::Inventory(Default::default())
    }

    /// Gets the default VecCoord.
    pub fn new_vec_coord() -> Self {
        Self::VecCoord(Default::default())
    }

    pub fn as_inventory_mut(&mut self) -> Option<&mut Inventory> {
        if let Self::Inventory(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_coord_mut(&mut self) -> Option<&mut TileCoord> {
        if let Self::Coord(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_bool_mut(&mut self) -> Option<&mut bool> {
        if let Self::Bool(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_vec_coord_mut(&mut self) -> Option<&mut Vec<TileCoord>> {
        if let Self::VecCoord(v) = self {
            return Some(v);
        }
        None
    }

    /// Gets a mutable reference to the tile's ID, or None.
    pub fn as_id_mut(&mut self) -> Option<&mut Id> {
        if let Self::Id(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_vec_id_mut(&mut self) -> Option<&mut Vec<Id>> {
        if let Self::VecId(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_amount_mut(&mut self) -> Option<&mut ItemAmount> {
        if let Self::Amount(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_inventory(&self) -> Option<&Inventory> {
        if let Self::Inventory(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_coord(&self) -> Option<&TileCoord> {
        if let Self::Coord(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_bool(&self) -> Option<&bool> {
        if let Self::Bool(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_vec_coord(&self) -> Option<&Vec<TileCoord>> {
        if let Self::VecCoord(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_id(&self) -> Option<&Id> {
        if let Self::Id(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_vec_id(&self) -> Option<&Vec<Id>> {
        if let Self::VecId(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_amount(&self) -> Option<&ItemAmount> {
        if let Self::Amount(v) = self {
            return Some(v);
        }
        None
    }

    pub fn rhai_inventory(self) -> Dynamic {
        if let Self::Inventory(v) = self {
            return Dynamic::from(v);
        }
        Dynamic::UNIT
    }

    pub fn rhai_coord(self) -> Dynamic {
        if let Self::Coord(v) = self {
            return Dynamic::from(v);
        }
        Dynamic::UNIT
    }

    pub fn rhai_bool(self) -> Dynamic {
        if let Self::Bool(v) = self {
            return Dynamic::from_bool(v);
        }
        Dynamic::UNIT
    }

    pub fn rhai_vec_coord(self) -> Dynamic {
        if let Self::VecCoord(v) = self {
            return Dynamic::from_iter(v);
        }
        Dynamic::UNIT
    }

    pub fn rhai_id(self) -> Dynamic {
        if let Self::Id(v) = self {
            return Dynamic::from_int(v.into());
        }
        Dynamic::UNIT
    }

    pub fn rhai_vec_id(self) -> Dynamic {
        if let Self::VecId(v) = self {
            return Dynamic::from_iter(v);
        }
        Dynamic::UNIT
    }

    pub fn rhai_amount(self) -> Dynamic {
        if let Self::Amount(v) = self {
            return Dynamic::from_int(v);
        }
        Dynamic::UNIT
    }

    pub fn rhai_value(self) -> Dynamic {
        match self {
            v @ Data::Inventory(_) => v.rhai_inventory(),
            v @ Data::Coord(_) => v.rhai_coord(),
            v @ Data::VecCoord(_) => v.rhai_vec_coord(),
            v @ Data::Id(_) => v.rhai_id(),
            v @ Data::VecId(_) => v.rhai_vec_id(),
            v @ Data::Amount(_) => v.rhai_amount(),
            v @ Data::Bool(_) => v.rhai_bool(),
        }
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
                    interner.resolve(*key).map(|key| {
                        (
                            key.to_string(),
                            match value {
                                Data::Inventory(v) => DataRaw::Inventory(v.to_raw(interner)),
                                Data::Coord(v) => DataRaw::Coord(*v),
                                Data::VecCoord(v) => DataRaw::VecCoord(v.clone()),
                                Data::Id(v) => {
                                    // TODO dont unwrap
                                    DataRaw::Id(IdRaw::parse(interner.resolve(*v).unwrap()))
                                }
                                Data::VecId(v) => DataRaw::VecId(
                                    v.iter()
                                        .map(|id| IdRaw::parse(interner.resolve(*id).unwrap()))
                                        .collect(),
                                ),
                                Data::Amount(v) => DataRaw::Amount(*v),
                                Data::Bool(v) => DataRaw::Bool(*v),
                            },
                        )
                    })
                })
                .collect(),
        )
    }

    fn rhai_parse(ty: ImmutableString) -> Option<Data> {
        let ty = ty.to_lowercase();

        match ty.as_str() {
            "inventory" => Some(Data::new_inventory()),
            "veccoord" => Some(Data::new_vec_coord()),
            "bool" => Some(Data::Bool(false)),
            "amount" => Some(Data::Amount(0)),
            "coord" => Some(Data::Coord(TileCoord::ZERO)),
            _ => None,
        }
    }

    pub fn rhai_get(&mut self, id: Id) -> Dynamic {
        if let Some(v) = self.get(&id).cloned() {
            v.rhai_value()
        } else {
            Dynamic::UNIT
        }
    }

    pub fn rhai_set(&mut self, id: Id, value: Dynamic) {
        self.0.insert(id, value.try_into().unwrap());
    }

    pub fn rhai_get_or_insert(&mut self, id: Id, ty: ImmutableString) -> Dynamic {
        self.0
            .entry(id)
            .or_insert_with(|| Self::rhai_parse(ty).unwrap())
            .clone()
            .rhai_value()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DataRaw {
    Inventory(InventoryRaw),
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    Id(IdRaw),
    VecId(Vec<IdRaw>),
    Amount(ItemAmount),
    Bool(bool),
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

    pub fn intern_to_data(&self, resource_man: &mut ResourceManager) -> DataMap {
        DataMap(
            self.0
                .iter()
                .map(|(key, value)| {
                    (
                        IdRaw::parse(key).to_id(&mut resource_man.interner),
                        match value {
                            DataRaw::Inventory(v) => Data::Inventory(v.to_inventory(resource_man)),
                            DataRaw::Coord(v) => Data::Coord(*v),
                            DataRaw::VecCoord(v) => Data::VecCoord(v.clone()),
                            DataRaw::Id(v) => {
                                Data::Id(resource_man.interner.get_or_intern(v.to_string()))
                            }
                            DataRaw::VecId(v) => Data::VecId(
                                v.iter()
                                    .map(|id| resource_man.interner.get_or_intern(id.to_string()))
                                    .collect(),
                            ),
                            DataRaw::Amount(v) => Data::Amount(*v),
                            DataRaw::Bool(v) => Data::Bool(*v),
                        },
                    )
                })
                .collect(),
        )
    }

    pub fn to_data(&self, resource_man: &ResourceManager) -> DataMap {
        DataMap(
            self.0
                .iter()
                .flat_map(|(key, value)| {
                    resource_man
                        .interner
                        .get(IdRaw::parse(key).to_string())
                        .map(|key| {
                            (
                                key,
                                match value {
                                    DataRaw::Inventory(v) => {
                                        Data::Inventory(v.to_inventory(resource_man))
                                    }
                                    DataRaw::Coord(v) => Data::Coord(*v),
                                    DataRaw::VecCoord(v) => Data::VecCoord(v.clone()),
                                    DataRaw::Id(v) => {
                                        Data::Id(resource_man.interner.get(v.to_string()).unwrap())
                                    }
                                    DataRaw::VecId(v) => Data::VecId(
                                        v.iter()
                                            .flat_map(|id| {
                                                resource_man.interner.get(id.to_string())
                                            })
                                            .collect(),
                                    ),
                                    DataRaw::Amount(v) => Data::Amount(*v),
                                    DataRaw::Bool(v) => Data::Bool(*v),
                                },
                            )
                        })
                })
                .collect(),
        )
    }
}
