use std::collections::{
    BTreeMap,
    btree_map::{self, Entry},
};

use hashbrown::{HashMap, HashSet};

use crate::{
    game::{
        coord::{TileBounds, TileCoord},
        inventory::Inventory,
        item::{ItemAmount, ItemStack},
    },
    id::{Id, Interner},
    parse::id::{resolve_ids, resolve_map_id_item, resolve_map_item_id},
    rendering::colors::{ColorExt, Rgba},
};

/// Represents a generic Data type.
#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    Inventory(Inventory),
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    TileBounds(TileBounds),
    Id(Id),
    Color(Rgba),
    VecId(Vec<Id>),
    SetId(HashSet<Id>),
    Bool(bool),
    // TODO integer type
    Amount(ItemAmount),
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
}

impl Data {
    fn to_raw(self, interner: &Interner) -> raw::DataRaw {
        match self {
            Data::Id(v) => raw::DataRaw::Id(interner.resolve(v).unwrap().to_string()),
            Data::VecId(v) => raw::DataRaw::VecId(resolve_ids(v.into_iter(), interner).collect()),
            Data::SetId(v) => raw::DataRaw::SetId(resolve_ids(v.into_iter(), interner).collect()),
            Data::Amount(v) => raw::DataRaw::Amount(v),
            Data::Bool(v) => raw::DataRaw::Bool(v),
            Data::Color(v) => {
                let v = v.to_u8();
                raw::DataRaw::Color(const_hex::encode([v.r, v.g, v.b, v.a]))
            }
            Data::TileBounds(v) => raw::DataRaw::TileBounds(v),
            Data::TileMap(v) => raw::DataRaw::TileMap(
                resolve_map_item_id(v.iter().map(|(a, b)| (*a, *b)), interner).collect(),
            ),
            Data::Inventory(v) => raw::DataRaw::Inventory(v.to_raw(interner)),
            Data::Coord(v) => raw::DataRaw::Coord(v),
            Data::VecCoord(v) => raw::DataRaw::VecCoord(v),
            Data::MapSetId(v) => raw::DataRaw::MapSetId(
                resolve_map_id_item(
                    v.into_iter()
                        .map(|(id, set)| (id, resolve_ids(set.into_iter(), interner).collect())),
                    interner,
                )
                .collect(),
            ),
        }
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

    pub fn to_raw(self, interner: &Interner) -> raw::DataMapRaw {
        raw::DataMapRaw(
            self.0
                .into_iter()
                .flat_map(|(id, value)| {
                    interner
                        .resolve(id)
                        .map(|id| (id.to_string(), value.to_raw(interner)))
                })
                .collect(),
        )
    }
}

pub mod raw {
    use std::collections::BTreeMap;

    use hashbrown::HashSet;
    use serde::{Deserialize, Serialize};
    use vek::Rgba;

    use crate::{
        game::{
            coord::{TileBounds, TileCoord, offset_coord_to_tile},
            generic::{Data, DataMap},
            inventory::raw::InventoryRaw,
            item::ItemAmount,
        },
        id::{Id, Interner},
        math::IVec2,
        parse::id::{
            parse_ids, parse_map_id_item, parse_map_item_id, try_parse_ids, try_parse_map_id_item,
            try_parse_map_item_id,
        },
        rendering::colors::{ColorExt, RgbaU8},
    };

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
        pub fn try_to_data(self, interner: &Interner) -> Option<Data> {
            Some(match self {
                DataRaw::Id(v) => Data::Id(Id::try_parse(&v, interner)?),
                DataRaw::VecId(v) => Data::VecId(try_parse_ids(v.into_iter(), interner).collect()),
                DataRaw::SetId(v) => Data::SetId(try_parse_ids(v.into_iter(), interner).collect()),
                DataRaw::TileMap(v) => Data::TileMap(
                    try_parse_map_item_id(
                        v.into_iter()
                            .map(|(coord, id)| (offset_coord_to_tile(coord.to_array()), id)),
                        interner,
                    )
                    .collect(),
                ),
                DataRaw::TileMapOffsetCoord(v) => Data::TileMap(
                    try_parse_map_item_id(
                        v.into_iter()
                            .map(|(coord, id)| (offset_coord_to_tile(coord.into_array()), id)),
                        interner,
                    )
                    .collect(),
                ),
                DataRaw::MapSetId(v) => Data::MapSetId(
                    try_parse_map_id_item(
                        v.into_iter().map(|(id, set)| {
                            (id, try_parse_ids(set.into_iter(), interner).collect())
                        }),
                        interner,
                    )
                    .collect(),
                ),
                DataRaw::Inventory(v) => Data::Inventory(v.try_to_inventory(interner)),
                DataRaw::Amount(v) => Data::Amount(v),
                DataRaw::Bool(v) => Data::Bool(v),
                DataRaw::Color(v) => {
                    let mut color = const_hex::decode(v).ok()?.into_iter();
                    Data::Color(Rgba::from_u8(RgbaU8 {
                        r: color.next()?,
                        g: color.next()?,
                        b: color.next()?,
                        a: color.next().unwrap_or(255),
                    }))
                }
                DataRaw::Coord(v) => Data::Coord(v),
                DataRaw::VecCoord(v) => Data::VecCoord(v),
                DataRaw::VecOffsetCoord(v) => Data::VecCoord(
                    v.iter()
                        .map(|v| offset_coord_to_tile(v.into_array()))
                        .collect(),
                ),
                DataRaw::TileBounds(v) => Data::TileBounds(v),
            })
        }

        pub fn to_data(self, interner: &mut Interner, namespace: Option<&str>) -> Option<Data> {
            Some(match self {
                DataRaw::Id(v) => Data::Id(Id::parse(&v, interner, namespace)?),
                DataRaw::VecId(v) => {
                    Data::VecId(parse_ids(v.into_iter(), interner, namespace).collect())
                }
                DataRaw::SetId(v) => {
                    Data::SetId(parse_ids(v.into_iter(), interner, namespace).collect())
                }
                DataRaw::TileMap(v) => {
                    Data::TileMap(parse_map_item_id(v.into_iter(), interner, namespace).collect())
                }
                DataRaw::TileMapOffsetCoord(v) => Data::TileMap(
                    parse_map_item_id(
                        v.into_iter()
                            .map(|(coord, id)| (offset_coord_to_tile(coord.into_array()), id)),
                        interner,
                        namespace,
                    )
                    .collect(),
                ),
                DataRaw::MapSetId(v) => Data::MapSetId(
                    parse_map_id_item(
                        v.into_iter()
                            .map(|(id, set)| {
                                (
                                    id,
                                    parse_ids(set.into_iter(), interner, namespace)
                                        .collect::<HashSet<_, _>>(),
                                )
                            })
                            .collect::<Vec<_>>()
                            .into_iter(),
                        interner,
                        namespace,
                    )
                    .collect(),
                ),
                DataRaw::Inventory(v) => Data::Inventory(v.to_inventory(interner, namespace)),
                rest => rest.try_to_data(interner)?,
            })
        }
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct DataMapRaw(pub(crate) BTreeMap<String, DataRaw>);

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

        pub fn try_to_data(self, interner: &Interner) -> DataMap {
            DataMap(
                self.0
                    .into_iter()
                    .flat_map(|(id, value)| {
                        Id::try_parse(&id, interner)
                            .and_then(|id| value.try_to_data(interner).map(|v| (id, v)))
                    })
                    .collect(),
            )
        }

        pub fn to_data(self, interner: &mut Interner, namespace: Option<&str>) -> DataMap {
            DataMap(
                self.0
                    .into_iter()
                    .flat_map(|(id, value)| {
                        Id::parse(&id, interner, namespace).zip(value.to_data(interner, namespace))
                    })
                    .collect(),
            )
        }
    }
}
