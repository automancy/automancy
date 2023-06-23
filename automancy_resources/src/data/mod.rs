use std::collections::BTreeMap;

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
    /// The tile entity's inventory.
    Inventory(Inventory),
    /// The coordinates of the tile.
    Coord(TileCoord),
    /// The tile's ID.
    VecCoord(Vec<TileCoord>),
    Id(Id),
    VecId(Vec<Id>),
    Amount(ItemAmount),
    Bool(bool),
}

impl Data {
    /// Gets the default Inventory.
    pub fn inventory() -> Self {
        Self::Inventory(Default::default())
    }

    pub fn vec_coord() -> Self {
        Self::VecCoord(Default::default())
    }

    /// Gets a mutable reference to  the tile's Inventory, or None.
    pub fn as_inventory_mut(&mut self) -> Option<&mut Inventory> {
        if let Self::Inventory(v) = self {
            return Some(v);
        }
        None
    }

    /// Gets a mutable reference to  the tile's coordinates, or None.
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

    /// Gets an immutable reference to  the tile's Inventory, or None.
    pub fn as_inventory(&self) -> Option<&Inventory> {
        if let Self::Inventory(v) = self {
            return Some(v);
        }
        None
    }

    /// Gets an immutable reference to  the tile's coordinates, or None.
    pub fn as_coord(&self) -> Option<&TileCoord> {
        if let Self::Coord(v) = self {
            return Some(v);
        }
        None
    }

    /// Gets an immutable reference to the tile's ID, or None.
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
}

#[derive(Clone, Debug, Default)]
pub struct DataMap(pub BTreeMap<Id, Data>);

impl DataMap {
    pub fn get(&self, id: &Id) -> Option<&Data> {
        self.0.get(id)
    }

    pub fn get_mut(&mut self, id: &Id) -> Option<&mut Data> {
        self.0.get_mut(id)
    }

    pub fn remove(&mut self, id: &Id) -> Option<Data> {
        self.0.remove(id)
    }
}

impl DataMap {
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
pub struct DataMapRaw(pub HashMap<String, DataRaw>);

impl DataMapRaw {
    pub fn get(&self, id: &str) -> Option<&DataRaw> {
        self.0.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut DataRaw> {
        self.0.get_mut(id)
    }
}

impl DataMapRaw {
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
