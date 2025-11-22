use std::collections::{
    BTreeMap,
    btree_map::{self, Entry},
};

use hashbrown::{HashMap, HashSet};

use crate::{
    game::{
        coord::{TileBounds, TileCoord},
        generic::serailize::IdMap,
        inventory::{Inventory, ItemAmount, ItemStack},
    },
    id::{Id, Interner},
    rendering::colors::Rgba,
};

/// Represents a generic Data type.
#[derive(Debug, Clone, PartialEq)]
pub enum Datum {
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    // TODO integer type
    Amount(ItemAmount),
    Bool(bool),
    TileBounds(TileBounds),
    Color(Rgba),
    Inventory(Inventory),
    Id(Id),
    VecId(Vec<Id>),
    SetId(HashSet<Id>),
    TileMap(HashMap<TileCoord, Id>),
    MapSetId(HashMap<Id, HashSet<Id>>),
}

impl Datum {
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

#[repr(transparent)]
#[derive(Debug, Clone, Default)]
pub struct DataMap(BTreeMap<Id, Datum>);

impl IntoIterator for DataMap {
    type Item = (Id, Datum);

    type IntoIter = btree_map::IntoIter<Id, Datum>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl DataMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keys(&self) -> impl Iterator<Item = &Id> {
        self.0.keys()
    }

    pub fn get(&self, id: Id) -> Option<&Datum> {
        self.0.get(&id)
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut Datum> {
        self.0.get_mut(&id)
    }

    pub fn set(&mut self, id: Id, datum: Datum) {
        self.0.insert(id, datum);
    }

    pub fn remove(&mut self, id: Id) -> Option<Datum> {
        self.0.remove(&id)
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn entry(&mut self, id: Id) -> Entry<'_, Id, Datum> {
        self.0.entry(id)
    }

    pub fn inventory_mut(&mut self, id: Id) -> &mut Inventory {
        if !matches!(self.get(id), Some(&Datum::Inventory(..))) {
            self.set(id, Datum::Inventory(Default::default()));
        }

        if let Some(Datum::Inventory(v)) = self.get_mut(id) {
            v
        } else {
            panic!();
        }
    }

    pub fn set_id_mut(&mut self, id: Id) -> &mut HashSet<Id> {
        if !matches!(self.get(id), Some(&Datum::SetId(..))) {
            self.set(id, Datum::SetId(Default::default()));
        }

        if let Some(Datum::SetId(v)) = self.get_mut(id) {
            v
        } else {
            panic!();
        }
    }

    pub fn bool_or_default(&self, id: Id, default: bool) -> bool {
        if let Some(Datum::Bool(v)) = self.get(id) { *v } else { default }
    }

    pub fn bool_mut_or_default(&mut self, id: Id, default: bool) -> bool {
        if !matches!(self.get(id), Some(&Datum::Bool(..))) {
            self.set(id, Datum::Bool(default));
        }

        if let Some(Datum::Bool(v)) = self.get(id) {
            *v
        } else {
            panic!();
        }
    }

    pub fn contains_id(&self, key: Id, id: Id) -> bool {
        if let Some(v) = self.get(key) {
            match v {
                Datum::Inventory(v) => v.contains_key(&id),
                Datum::Id(v) => *v == id,
                Datum::VecId(v) => v.contains(&id),
                Datum::SetId(v) => v.contains(&id),
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn contains_stack(&mut self, key: Id, stack: ItemStack) -> bool {
        if let Some(Datum::Inventory(v)) = self.get_mut(key) {
            v.contains(stack)
        } else {
            false
        }
    }

    pub fn into_raw(self, id_map: &mut IdMap, interner: &Interner) -> serailize::DataMapRaw {
        let mut map = serailize::DataMapRaw::default();

        for (id, datum) in self {
            id_map.insert(id, interner);

            map.insert(id, datum.into_raw(id_map, interner));
        }

        map
    }
}

pub mod serailize {
    use core::ops::{Deref, DerefMut};
    use std::collections::BTreeMap;

    use hashbrown::{HashMap, HashSet};
    use serde::{Deserialize, Serialize};
    use thiserror::Error;
    use vek::Rgba;

    use crate::{
        game::{
            coord::{TileBounds, TileCoord},
            generic::{DataMap, Datum},
            inventory::{ItemAmount, serialize::InventoryRaw},
        },
        id::{Id, Interner},
        rendering::colors::ColorExt,
    };

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum DatumRaw {
        Coord(TileCoord),
        VecCoord(Vec<TileCoord>),
        Amount(ItemAmount),
        Bool(bool),
        TileBounds(TileBounds),
        Color(String),
        Inventory(InventoryRaw),
        Id(Id),
        VecId(Vec<Id>),
        SetId(Vec<Id>),
        TileMap(Vec<(TileCoord, Id)>),
        MapSetId(Vec<(Id, Vec<Id>)>),
    }

    impl Datum {
        pub fn into_raw(self, id_map: &mut IdMap, interner: &Interner) -> DatumRaw {
            match self {
                Datum::Coord(v) => DatumRaw::Coord(v),
                Datum::VecCoord(v) => DatumRaw::VecCoord(v),
                Datum::Amount(v) => DatumRaw::Amount(v),
                Datum::Bool(v) => DatumRaw::Bool(v),
                Datum::TileBounds(v) => DatumRaw::TileBounds(v),
                Datum::Color(v) => DatumRaw::Color(v.encode()),
                Datum::Inventory(v) => DatumRaw::Inventory(v.into_raw(id_map, interner)),
                Datum::Id(v) => {
                    id_map.insert(v, interner);

                    DatumRaw::Id(v)
                }
                Datum::VecId(v) => {
                    let mut r = Vec::new();

                    for id in v {
                        id_map.insert(id, interner);

                        r.push(id);
                    }

                    DatumRaw::VecId(r)
                }
                Datum::SetId(v) => {
                    let mut r = Vec::new();

                    for id in v {
                        id_map.insert(id, interner);

                        r.push(id);
                    }

                    DatumRaw::SetId(r)
                }

                Datum::TileMap(v) => {
                    let mut r = Vec::new();

                    for (coord, id) in v {
                        id_map.insert(id, interner);

                        r.push((coord, id));
                    }

                    DatumRaw::TileMap(r)
                }
                Datum::MapSetId(v) => {
                    let mut r = Vec::new();

                    for (id, set) in v {
                        id_map.insert(id, interner);

                        for id in &set {
                            id_map.insert(*id, interner);
                        }

                        r.push((id, set.into_iter().collect::<Vec<_>>()));
                    }

                    DatumRaw::MapSetId(r)
                }
            }
        }
    }

    impl DatumRaw {
        pub fn into_datum(self, id_map: &IdMap, interner: &Interner) -> Result<Datum, IdMapError> {
            Ok(match self {
                DatumRaw::Amount(v) => Datum::Amount(v),
                DatumRaw::Bool(v) => Datum::Bool(v),
                DatumRaw::Coord(v) => Datum::Coord(v),
                DatumRaw::VecCoord(v) => Datum::VecCoord(v),
                DatumRaw::TileBounds(v) => Datum::TileBounds(v),
                DatumRaw::Color(v) => Datum::Color(Rgba::decode(v)),
                DatumRaw::Id(v) => Datum::Id(id_map.resolve(v, interner)?),
                DatumRaw::VecId(v) => {
                    let mut r = Vec::new();

                    for unmapped_id in v {
                        r.push(id_map.resolve(unmapped_id, interner)?);
                    }

                    Datum::VecId(r)
                }
                DatumRaw::SetId(v) => {
                    let mut r = HashSet::new();

                    for unmapped_id in v {
                        r.insert(id_map.resolve(unmapped_id, interner)?);
                    }

                    Datum::SetId(r)
                }
                DatumRaw::TileMap(v) => {
                    let mut r = HashMap::new();

                    for (coord, unmapped_id) in v {
                        r.insert(coord, id_map.resolve(unmapped_id, interner)?);
                    }

                    Datum::TileMap(r)
                }
                DatumRaw::MapSetId(v) => {
                    let mut r = HashMap::new();

                    for (unmapped_id, unmapped_set) in v {
                        let mut ids = HashSet::new();

                        for unmapped_id in unmapped_set {
                            ids.insert(id_map.resolve(unmapped_id, interner)?);
                        }

                        r.insert(id_map.resolve(unmapped_id, interner)?, ids);
                    }

                    Datum::MapSetId(r)
                }
                DatumRaw::Inventory(v) => Datum::Inventory(v.into_inventory(id_map, interner)?),
            })
        }
    }

    #[derive(Debug, Error)]
    pub enum IdMapError {
        #[error("IdMap is missing the Id->String mapping for {0:?}")]
        MapMissingId(Id),
        #[error("Interner is missing the String->Id mapping for {0}")]
        InternerMissingStringId(String),
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct IdMap {
        map: HashMap<Id, String>,
    }

    impl Default for IdMap {
        fn default() -> Self {
            Self::new()
        }
    }

    impl IdMap {
        pub fn new() -> Self {
            Self { map: Default::default() }
        }

        pub fn resolve(&self, unmapped_id: Id, interner: &Interner) -> Result<Id, IdMapError> {
            let string_id = self.map.get(&unmapped_id).ok_or(IdMapError::MapMissingId(unmapped_id))?;

            let id = interner
                .get(string_id)
                .ok_or_else(|| IdMapError::InternerMissingStringId(string_id.clone()))?;

            Ok(id)
        }

        pub fn intern(&self, unmapped_id: Id, interner: &mut Interner) -> Result<Id, IdMapError> {
            let string_id = self.map.get(&unmapped_id).ok_or(IdMapError::MapMissingId(unmapped_id))?;

            let id = interner.get_or_intern(string_id);

            Ok(id)
        }

        pub fn insert(&mut self, id: Id, interner: &Interner) {
            if !self.map.contains_key(&id) {
                self.map.insert(id, interner.resolve(id).unwrap().to_string());
            }
        }
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct DataMapRaw(BTreeMap<Id, DatumRaw>);

    impl Deref for DataMapRaw {
        type Target = BTreeMap<Id, DatumRaw>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for DataMapRaw {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl From<BTreeMap<Id, DatumRaw>> for DataMapRaw {
        fn from(value: BTreeMap<Id, DatumRaw>) -> Self {
            Self(value)
        }
    }

    impl DataMapRaw {
        pub fn into_inner(self) -> BTreeMap<Id, DatumRaw> {
            self.0
        }

        pub fn into_data(self, id_map: &IdMap, interner: &Interner) -> Result<DataMap, IdMapError> {
            let mut map = DataMap::new();

            for (unmapped_id, raw_datum) in self.0.into_iter() {
                let id = id_map.resolve(unmapped_id, interner)?;

                let datum = raw_datum.into_datum(id_map, interner)?;

                map.set(id, datum);
            }

            Ok(map)
        }
    }
}

pub mod deserialize {
    use core::ops::{Deref, DerefMut};
    use std::collections::BTreeMap;

    use hashbrown::HashMap;
    use serde::Deserialize;

    use crate::{
        game::{
            coord::{OffsetCoord, TileBounds, TileCoord, TileUnit},
            generic::{DataMap, Datum},
            inventory::{ItemAmount, deserialize::InventoryStr},
        },
        id::{
            Interner,
            deserialize::{StrId, StrIdParseError},
            parse::{parse_ids, parse_map_item_id},
        },
        rendering::colors::{ColorExt, Rgba},
    };

    #[derive(Debug, Clone, Deserialize)]
    pub enum DatumStr {
        Coord(TileCoord),
        VecCoord(Vec<TileCoord>),
        Amount(ItemAmount),
        Bool(bool),
        TileBounds(TileBounds),
        Color(String),
        Inventory(InventoryStr),
        Id(StrId),
        VecId(Vec<StrId>),
        VecOffsetCoord(Vec<[TileUnit; 2]>),
        SetId(Vec<StrId>),
        TileMap(Vec<(TileCoord, StrId)>),
        TileMapOffsetCoord(Vec<([TileUnit; 2], StrId)>),
        MapSetId(Vec<(StrId, Vec<StrId>)>),
    }

    impl DatumStr {
        pub fn into_datum(self, interner: &mut Interner, fallback_namespace: Option<&str>) -> Result<Datum, StrIdParseError> {
            Ok(match self {
                DatumStr::Amount(v) => Datum::Amount(v),
                DatumStr::Bool(v) => Datum::Bool(v),
                DatumStr::Coord(v) => Datum::Coord(v),
                DatumStr::VecCoord(v) => Datum::VecCoord(v),
                DatumStr::VecOffsetCoord(v) => Datum::VecCoord(
                    v.into_iter()
                        .map(|[col, row]| TileCoord::from_offset(OffsetCoord::new(col, row)))
                        .collect(),
                ),
                DatumStr::TileBounds(v) => Datum::TileBounds(v),
                DatumStr::Color(v) => Datum::Color(Rgba::decode(v)),
                DatumStr::Id(v) => Datum::Id(v.into_id(interner, fallback_namespace)?),
                DatumStr::VecId(v) => Datum::VecId(parse_ids(v.into_iter(), interner, fallback_namespace).try_collect()?),
                DatumStr::SetId(v) => Datum::SetId(parse_ids(v.into_iter(), interner, fallback_namespace).try_collect()?),
                DatumStr::TileMap(v) => Datum::TileMap(parse_map_item_id(v.into_iter(), interner, fallback_namespace).try_collect()?),
                DatumStr::TileMapOffsetCoord(v) => Datum::TileMap(
                    parse_map_item_id(
                        v.into_iter()
                            .map(|([col, row], id)| (TileCoord::from_offset(OffsetCoord::new(col, row)), id)),
                        interner,
                        fallback_namespace,
                    )
                    .try_collect()?,
                ),
                DatumStr::MapSetId(v) => {
                    let mut r = HashMap::new();

                    for (id, set) in v {
                        r.insert(
                            id.into_id(interner, fallback_namespace)?,
                            parse_ids(set.into_iter(), interner, fallback_namespace).try_collect()?,
                        );
                    }

                    Datum::MapSetId(r)
                }
                DatumStr::Inventory(v) => Datum::Inventory(v.into_inventory(interner, fallback_namespace)?),
            })
        }
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(transparent)]
    pub struct DataMapStr(BTreeMap<StrId, DatumStr>);

    impl Deref for DataMapStr {
        type Target = BTreeMap<StrId, DatumStr>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for DataMapStr {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl From<BTreeMap<StrId, DatumStr>> for DataMapStr {
        fn from(value: BTreeMap<StrId, DatumStr>) -> Self {
            Self(value)
        }
    }

    impl DataMapStr {
        pub fn into_data(self, interner: &mut Interner, fallback_namespace: Option<&str>) -> Result<DataMap, StrIdParseError> {
            let mut map = DataMap::new();

            for (id, raw_datum) in self.0.into_iter() {
                map.set(
                    id.into_id(interner, fallback_namespace)?,
                    raw_datum.into_datum(interner, fallback_namespace)?,
                );
            }

            Ok(map)
        }
    }
}
