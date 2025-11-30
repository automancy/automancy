use std::collections::{BTreeMap, btree_map};

use crate::{
    game::{
        coord::{TileCoord, TileCoordBounds},
        inventory::{Inventory, ItemStack},
    },
    id::Id,
    math::{Float, Int, UInt},
    rendering::colors::Rgba,
};

macro_rules! impl_data_getters {
    ($data_type:ty {
        $( $fn_name:ident: $datum_name:ident($datum_type:ty) ),*
        $(,)?
    }) => {
        $(paste::paste! {
            impl $data_type {
                pub fn $fn_name(&self, id: Id) -> Option<&$datum_type> {
                    if let Some(Datum::$datum_name(v)) = self.get(id) {
                        Some(v)
                    } else {
                        None
                    }
                }

                pub fn [<$fn_name _then>]<R>(&self, id: Id, f: impl FnOnce(&$datum_type) -> R) -> Option<R> {
                    if let Some(Datum::$datum_name(v)) = self.get(id) {
                        Some(f(v))
                    } else {
                        None
                    }
                }

                pub fn [<$fn_name _mut_or>](&mut self, id: Id, default: $datum_type) -> &mut $datum_type {
                    if !matches!(self.get(id), Some(&Datum::$datum_name(..))) {
                        self.set(id, Datum::$datum_name(default));
                    }

                    if let Some(Datum::$datum_name(v)) = self.get_mut(id) {
                        v
                    } else {
                        unreachable!();
                    }
                }

                pub fn [<$fn_name _mut>](&mut self, id: Id) -> &mut $datum_type
                    where for<'a> $datum_type: Default
                {
                    self.[<$fn_name _mut_or>](id, Default::default())
                }
            }
        })+
    }
}

macro_rules! impl_data_of_collections {
    ($data_type:ty {
        $( $fn_name:ident: {
            $datum_name_single:ident($datum_type_single:ty),
            $datum_name_set:ident($datum_type_set:ty),
        } ),*
        $(,)?
    }) => {
        $(paste::paste! {
            impl $data_type {
                pub fn [<contains_ $fn_name>](&self, key: Id, v: $datum_type_single) -> bool {
                    if let Some(r) = self.get(key) {
                        match r {
                            Datum::$datum_name_single(r) => {
                                return *r == v;
                            }
                            Datum::$datum_name_set(r) => {
                                return r.contains(&v);
                            }
                            _ => {},
                        }
                    }

                    false
                }

                pub fn [<$fn_name s>](&self, key: Id) -> $datum_type_set {
                    if let Some(r) = self.get(key) {
                        match r {
                            Datum::$datum_name_single(r) => {
                                return FromIterator::from_iter([*r]);
                            }
                            Datum::$datum_name_set(r) => {
                                return r.clone()
                            }
                            _ => {},
                        }
                    }

                    Default::default()
                }

                pub fn [<add_ $fn_name>](&mut self, key: Id, v: $datum_type_single) {
                    if let Some(r) = self.get_mut(key) {
                        match r {
                            Datum::$datum_name_single(r) => {
                                let r = *r;
                                self.set(key, Datum::$datum_name_set(FromIterator::from_iter([r, v])));
                            }
                            Datum::$datum_name_set(r) => {
                                r.insert(v);
                            }
                            _ => {},
                        }
                    } else {
                        self.set(key, Datum::$datum_name_single(v));
                    }
                }

                pub fn [<remove_ $fn_name>](&mut self, key: Id, v: $datum_type_single) {
                    if let Some(r) = self.get_mut(key) {
                        match r {
                            Datum::$datum_name_single(r) => {
                                #[allow(clippy::collapsible_match)]
                                if *r == v {
                                    self.remove(key);
                                }
                            },
                            Datum::$datum_name_set(r) => {
                                r.remove(&v);
                                if r.len() == 1 {
                                    let r = std::mem::take(r).into_iter().next().unwrap();
                                    self.set(key, Datum::$datum_name_single(r));
                                }
                            },
                            _ => {},
                        }
                    }
                }
            }
        })+
    }
}

macro_rules! define_datum {
    (
        ids: [
            $( $id_name:ident ),*
            $(,)?
        ];

        others: {
            $( $name:ident($type:ty) ),*
            $(,)?
        };

        handle_change($self:ident, $key:ident): {
            add: {
                $( $handle_add_pat:pat => $handle_add_expr: expr, )*
                $(,)?
            };

            sub: {
                $( $handle_sub_pat:pat => $handle_sub_expr: expr, )*
                $(,)?
            };
        };
    ) => {
        paste::paste! {
            /// Represents a generic Data type.
            #[must_use]
            #[derive(Debug, Clone)]
            pub enum Datum {
                $(
                                 $id_name($crate::id::$id_name),
                         [<Set $id_name>]($crate::id_map::IdSet<$crate::id::$id_name>),
                      [<MapSet $id_name>]($crate::id_map::IdMap<$crate::id::$id_name, $crate::id_map::IdSet<$crate::id::$id_name>>),
                    [<MapCoord $id_name>](::std::collections::BTreeMap<$crate::game::coord::TileCoord, $crate::id::$id_name>),
                )*
                $(
                    $name($type),
                )*
            }

            impl_data_getters! {
                DataMap {
                    $(
                                   [<$id_name:snake>]:              $id_name($crate::id::$id_name),
                              [<set_ $id_name:snake>]:      [<Set $id_name>]($crate::id_map::IdSet<$crate::id::$id_name>),
                          [<map_set_ $id_name:snake>]:   [<MapSet $id_name>]($crate::id_map::IdMap<$crate::id::$id_name, $crate::id_map::IdSet<$crate::id::$id_name>>),
                        [<map_coord_ $id_name:snake>]: [<MapCoord $id_name>](::std::collections::BTreeMap<$crate::game::coord::TileCoord, $crate::id::$id_name>),
                    )*
                    $(
                        [<$name:snake>]: $name($type),
                    )*
                }
            }

            impl_data_of_collections! {
                DataMap {
                    $(
                        [<$id_name:snake>]: {
                            $id_name($crate::id::$id_name),
                            [<Set $id_name>]($crate::id_map::IdSet<$crate::id::$id_name>),
                        },
                    )*
                }
            }

            impl DataMap {
                pub fn handle_change(&mut $self, change: DatumChange) {
                    match change {
                        DatumChange::Add($key, datum) => match datum {
                            $(
                                Datum::$id_name(v) => {
                                    $self.[<add_ $id_name:snake>]($key, v);
                                },
                                Datum::[<Set $id_name>](v) => {
                                    $self.[<set_ $id_name:snake _mut>]($key).extend(v);
                                },
                                Datum::[<MapSet $id_name>](v) => {
                                    let id_map = $self.[<map_set_ $id_name:snake _mut>]($key);
                                    for (id, set) in v {
                                        id_map.entry(id).or_default().extend(set);
                                    }
                                },
                                Datum::[<MapCoord $id_name>](v) => {
                                    $self.[<map_coord_ $id_name:snake _mut>]($key).extend(v);
                                },
                            )*

                            $(
                                $handle_add_pat => $handle_add_expr,
                            )*
                        },
                        DatumChange::Sub($key, datum) => match datum {
                            $(
                                Datum::$id_name(v) => {
                                    $self.[<remove_ $id_name:snake>]($key, v);
                                },
                                Datum::[<Set $id_name>](v) => {
                                    let set = $self.[<set_ $id_name:snake _mut>]($key);
                                    for id in v {
                                        set.remove(&id);
                                    }
                                },
                                Datum::[<MapSet $id_name>](v) => {
                                    let id_map = $self.[<map_set_ $id_name:snake _mut>]($key);
                                    for (id, set) in v {
                                        for v in set {
                                            id_map.entry(id).or_default().remove(&v);
                                        }
                                    }
                                },
                                Datum::[<MapCoord $id_name>](v) => {
                                    let map = $self.[<map_coord_ $id_name:snake _mut>]($key);
                                    for (coord, id) in v {
                                        if map.get(&coord) == Some(&id) {
                                            map.remove(&coord);
                                        }
                                    }
                                },
                            )*

                            $(
                                $handle_sub_pat => $handle_sub_expr,
                            )*
                        },
                        DatumChange::Set($key, datum) => $self.set($key, datum),
                    }
                }
            }
        }
    }
}

define_datum! {
    ids: [Id, TileId, ItemId, RecipeId, TagId, CategoryId, ResearchId, ScriptId, ModelId, RenderId];
    others: {
        TileCoord(TileCoord),
        VecTileCoord(Vec<TileCoord>),
        TileCoordBounds(TileCoordBounds),

        ItemStack(ItemStack),
        Inventory(Inventory),

        Color(Rgba),

        Int(Int),
        UInt(UInt),
        Float(Float),
        Bool(bool)
    };

    handle_change(self, key): {
        add: {
            datum @ (
                Datum::TileCoord(..)
                    | Datum::VecTileCoord(..)
                    | Datum::TileCoordBounds(..)
                    | Datum::Color(..)
                    | Datum::Bool(..)
            ) => {
                panic!("cannot add {datum:?} to `DataMap`");
            },

            Datum::ItemStack(v) => {
                self.add_stack(key, v);
            },
            Datum::Inventory(v) => {
                for v in v.into_stacks() {
                    self.add_stack(key, v);
                }
            },

            Datum::Int(v) => {
                *self.int_mut(key) += v;
            },
            Datum::UInt(v) => {
                *self.u_int_mut(key) += v;
            },
            Datum::Float(v) => {
                *self.float_mut(key) += v;
            },
        };

        sub: {
            datum @ (
                Datum::TileCoord(..)
                    | Datum::VecTileCoord(..)
                    | Datum::TileCoordBounds(..)
                    | Datum::Color(..)
                    | Datum::Bool(..)
            ) => {
                panic!("cannot subtract {datum:?} from `DataMap`");
            },

            Datum::ItemStack(v) => {
                self.remove_stack(key, v);
            },
            Datum::Inventory(v) => {
                for v in v.into_stacks() {
                    self.remove_stack(key, v);
                }
            },

            Datum::Int(v) => {
                *self.int_mut(key) -= v;
            },
            Datum::UInt(v) => {
                let uint = self.u_int_mut(key);
                *uint = uint.saturating_sub(v);
            },
            Datum::Float(v) => {
                *self.float_mut(key) -= v;
            },
        };
    };
}

#[derive(Debug, Clone)]
pub enum DatumChange {
    Add(Id, Datum),
    Sub(Id, Datum),
    Set(Id, Datum),
}

#[must_use]
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

    pub fn iter(&self) -> impl Iterator<Item = (&Id, &Datum)> {
        self.0.iter()
    }

    pub fn entry(&mut self, id: Id) -> btree_map::Entry<'_, Id, Datum> {
        self.0.entry(id)
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

    pub fn contains_stack(&self, key: Id, v: ItemStack) -> bool {
        if let Some(r) = self.get(key) {
            match r {
                Datum::ItemStack(r) => {
                    return r.id == v.id && r.amount >= v.amount;
                },
                Datum::Inventory(r) => {
                    return r.contains(v);
                },
                _ => {},
            }
        }

        false
    }
    pub fn stacks(&self, key: Id) -> Inventory {
        if let Some(r) = self.get(key) {
            match r {
                Datum::ItemStack(r) => return FromIterator::from_iter([*r]),
                Datum::Inventory(r) => return r.clone(),
                _ => {},
            }
        }

        Default::default()
    }
    pub fn add_stack(&mut self, key: Id, v: ItemStack) {
        if let Some(r) = self.get_mut(key) {
            match r {
                Datum::ItemStack(r) => {
                    if r.id == v.id {
                        r.amount += v.amount;
                    } else {
                        let r = *r;
                        self.set(key, Datum::Inventory(Inventory::from_iter([r, v])));
                    }
                },
                Datum::Inventory(r) => {
                    r.add(v.id, v.amount);
                },
                _ => {},
            }
        } else {
            self.set(key, Datum::ItemStack(v));
        }
    }
    pub fn remove_stack(&mut self, key: Id, v: ItemStack) {
        if let Some(r) = self.get_mut(key) {
            match r {
                Datum::ItemStack(r) => {
                    if r.id == v.id {
                        r.amount -= v.amount;
                        if r.amount == 0 {
                            self.remove(key);
                        }
                    } else {
                        let r = *r;
                        self.set(key, Datum::Inventory(Inventory::from_iter([r, -v])));
                    }
                },
                Datum::Inventory(r) => {
                    r.add(v.id, -v.amount);
                    if r.get(v.id) == 0 {
                        r.remove(&v.id);
                    }
                    if r.len() == 1 {
                        let r = std::mem::take(r).into_iter().next().unwrap();
                        self.set(key, Datum::ItemStack(ItemStack::from(r)));
                    }
                },
                _ => {},
            }
        } else {
            self.set(key, Datum::ItemStack(-v));
        }
    }
}

pub mod serailize {
    use core::ops::{Deref, DerefMut};
    use std::{borrow::Cow, collections::BTreeMap};

    use serde::{Deserialize, Serialize};
    use thiserror::Error;

    use crate::{
        game::{
            coord::{TileCoord, TileCoordBounds},
            generic::{DataMap, Datum},
            inventory::{ItemAmount, ItemStack, serialize::InventoryRaw},
        },
        id::{Id, IdInterner, ItemId, deserialize::StrIdParseError},
        id_map::{IdMap, IdSet},
        math::{Float, Int, UInt},
        rendering::colors,
    };

    #[derive(Debug, Error)]
    pub enum IdMappingError {
        #[error("serialization is missing the Id->String mapping for {0}")]
        MappingMissingId(Id),
        #[error("Interner is missing the String->Id mapping for '{0}'")]
        InternerMissingStringId(String),
        #[error(transparent)]
        StrIdParseError(#[from] StrIdParseError),
    }

    #[must_use]
    #[repr(transparent)]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct IdMapping {
        map: BTreeMap<Id, String>,
    }

    impl Default for IdMapping {
        fn default() -> Self {
            Self::new()
        }
    }

    impl IdMapping {
        pub fn new() -> Self {
            Self {
                map: Default::default(),
            }
        }

        pub fn resolve(&self, unmapped_id: Id, interner: &IdInterner) -> Result<Id, IdMappingError> {
            let id = self.map.get(&unmapped_id).ok_or(IdMappingError::MappingMissingId(unmapped_id))?;
            let id = interner
                .get(Cow::Borrowed(id))
                .ok_or_else(|| IdMappingError::InternerMissingStringId(id.to_string()))?;

            Ok(id)
        }

        pub fn get_or_intern(&self, unmapped_id: Id, interner: &mut IdInterner) -> Result<Id, IdMappingError> {
            let id = self.map.get(&unmapped_id).ok_or(IdMappingError::MappingMissingId(unmapped_id))?;
            let id = interner.get_or_intern(Cow::Borrowed(id), None)?;

            Ok(id)
        }

        pub fn insert(&mut self, id: Id, interner: &IdInterner) {
            self.map.entry(id).or_insert_with(|| interner.resolve(id).unwrap().to_string());
        }
    }

    macro_rules! define_datum_raw {
        (
            ids: [
                $( $id_name:ident ),*
                $(,)?
            ];

            others: {
                $( $name:ident($type:ty) ),*
                $(,)?
            };

            conv($id_map:ident, $interner:ident) {
                into_datum: {
                    $( $into_datum_others_pat:pat => $into_datum_others_expr: expr, )*
                    $(,)?
                };

                into_raw: {
                    $( $into_raw_others_pat:pat => $into_raw_others_expr: expr, )*
                    $(,)?
                };
            }
        ) => {
            paste::paste! {
                /// Represents a generic Data type.
                #[must_use]
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub enum DatumRaw {
                    $(
                                     $id_name($crate::id::$id_name),
                             [<Set $id_name>](::std::vec::Vec<$crate::id::$id_name>),
                          [<MapSet $id_name>](::std::vec::Vec<($crate::id::Id, ::std::vec::Vec<$crate::id::$id_name>)>),
                        [<MapCoord $id_name>](::std::vec::Vec<($crate::game::coord::TileCoord, $crate::id::$id_name)>),
                    )*
                    $(
                        $name($type),
                    )*
                }

                impl DatumRaw {
                    pub fn into_datum(self, $id_map: &IdMapping, $interner: &IdInterner) -> Result<Datum, IdMappingError> {
                        Ok(match self {
                            $(
                                DatumRaw::$id_name(v) => Datum::$id_name($id_map.resolve(v.into(), $interner)?.into()),
                                DatumRaw::[<Set $id_name>](v) => {
                                    let mut r = IdSet::new();

                                    for unmapped_id in v {
                                        r.insert($id_map.resolve(unmapped_id.into(), $interner)?.into());
                                    }

                                    Datum::[<Set $id_name>](r)
                                },
                                DatumRaw::[<MapSet $id_name>](v) => {
                                    let mut r = IdMap::new();

                                    for (unmapped_id, unmapped_set) in v {
                                        let mut ids = IdSet::new();

                                        for unmapped_id in unmapped_set {
                                            ids.insert($id_map.resolve(unmapped_id.into(), $interner)?.into());
                                        }

                                        r.insert($id_map.resolve(unmapped_id, $interner)?.into(), ids);
                                    }

                                    Datum::[<MapSet $id_name>](r)
                                },
                                DatumRaw::[<MapCoord $id_name>](v) => {
                                    let mut r = BTreeMap::new();

                                    for (coord, unmapped_id) in v {
                                        r.insert(coord, $id_map.resolve(unmapped_id.into(), $interner)?.into());
                                    }

                                    Datum::[<MapCoord $id_name>](r)
                                },
                            )*

                            $(
                                $into_datum_others_pat => $into_datum_others_expr,
                            )*
                        })
                    }
                }

                impl Datum {
                    pub fn into_raw(self, $id_map: &mut IdMapping, $interner: &IdInterner) -> DatumRaw {
                        match self {
                            $(
                                Datum::$id_name(v) => {
                                    $id_map.insert(v.into(), $interner);

                                    DatumRaw::$id_name(v)
                                },
                                Datum::[<Set $id_name>](v) => {
                                    let mut r = Vec::new();

                                    for id in v {
                                        $id_map.insert(id.into(), $interner);

                                        r.push(id);
                                    }

                                    DatumRaw::[<Set $id_name>](r)
                                },
                                Datum::[<MapSet $id_name>](v) => {
                                    let mut r = Vec::new();

                                    for (id, set) in v {
                                        $id_map.insert(id.into(), $interner);

                                        for &id in &set {
                                            $id_map.insert(id.into(), $interner);
                                        }

                                        r.push((id.into(), set.into_iter().collect::<Vec<_>>()));
                                    }

                                    DatumRaw::[<MapSet $id_name>](r)
                                },

                                Datum::[<MapCoord $id_name>](v) => {
                                    let mut r = Vec::new();

                                    for (coord, id) in v {
                                        $id_map.insert(id.into(), $interner);

                                        r.push((coord, id));
                                    }

                                    DatumRaw::[<MapCoord $id_name>](r)
                                },
                            )*
                            $(
                                $into_raw_others_pat => $into_raw_others_expr,
                            )*
                        }
                    }
                }
            }
        }
    }

    define_datum_raw! {
        ids: [Id, TileId, ItemId, RecipeId, TagId, CategoryId, ResearchId, ScriptId, ModelId, RenderId];
        others: {
            TileCoord(TileCoord),
            VecTileCoord(Vec<TileCoord>),
            TileCoordBounds(TileCoordBounds),

            ItemStack((Id, ItemAmount)),
            Inventory(InventoryRaw),

            Color(String),

            Int(Int),
            UInt(UInt),
            Float(Float),
            Bool(bool)
        };

        conv(id_map, interner) {
            into_datum: {
                DatumRaw::TileCoord(v) => Datum::TileCoord(v),
                DatumRaw::VecTileCoord(v) => Datum::VecTileCoord(v),
                DatumRaw::TileCoordBounds(v) => Datum::TileCoordBounds(v),

                DatumRaw::ItemStack((id, amount)) => {
                    Datum::ItemStack(ItemStack::from(
                        (
                            ItemId(id_map.resolve(id, interner)?),
                            amount
                        )
                    ))
                },
                DatumRaw::Inventory(v) => Datum::Inventory(v.into_inventory(id_map, interner)?),

                DatumRaw::Color(v) => Datum::Color(colors::decode_rgba(&v)),

                DatumRaw::Int(v) => Datum::Int(v),
                DatumRaw::UInt(v) => Datum::UInt(v),
                DatumRaw::Float(v) => Datum::Float(v),
                DatumRaw::Bool(v) => Datum::Bool(v),
            };

            into_raw: {
                Datum::TileCoord(v) => DatumRaw::TileCoord(v),
                Datum::VecTileCoord(v) => DatumRaw::VecTileCoord(v),
                Datum::TileCoordBounds(v) => DatumRaw::TileCoordBounds(v),

                Datum::ItemStack(v) => {
                    id_map.insert(*v.id, interner);
                    DatumRaw::ItemStack((*v.id, v.amount))
                },
                Datum::Inventory(v) => DatumRaw::Inventory(v.into_raw(id_map, interner)),

                Datum::Color(v) => DatumRaw::Color(colors::encode_rgba(v)),

                Datum::Int(v) => DatumRaw::Int(v),
                Datum::UInt(v) => DatumRaw::UInt(v),
                Datum::Float(v) => DatumRaw::Float(v),
                Datum::Bool(v) => DatumRaw::Bool(v),
            };
        }
    }

    #[must_use]
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

        pub fn into_data(self, id_map: &IdMapping, interner: &IdInterner) -> Result<DataMap, IdMappingError> {
            let mut map = DataMap::new();

            for (unmapped_id, raw_datum) in self.0.into_iter() {
                let id = id_map.resolve(unmapped_id, interner)?;

                let datum = raw_datum.into_datum(id_map, interner)?;

                map.set(id, datum);
            }

            Ok(map)
        }
    }

    impl DataMap {
        pub fn into_raw(self, id_map: &mut IdMapping, interner: &IdInterner) -> DataMapRaw {
            let mut map = DataMapRaw::default();

            for (id, datum) in self {
                id_map.insert(id, interner);

                map.insert(id, datum.into_raw(id_map, interner));
            }

            map
        }
    }
}

pub mod deserialize {
    use core::ops::{Deref, DerefMut};
    use std::collections::BTreeMap;

    use serde::Deserialize;

    use crate::{
        game::{
            coord::{OffsetCoord, TileCoord, TileCoordBounds, TileUnit},
            generic::{DataMap, Datum},
            inventory::{
                ItemStack,
                deserialize::{InventoryStr, ItemStackStr},
            },
        },
        id::{
            IdInterner, ItemId,
            deserialize::{StrId, StrIdParseError},
        },
        id_map::IdMap,
        math::{Float, Int, UInt},
        rendering::colors,
    };

    macro_rules! define_datum_str {
        (
            ids: [
                $( $id_name:ident ),*
                $(,)?
            ];

            others: {
                $( $name:ident($type:ty) ),*
                $(,)?
            };

            conv($interner:ident, $fallback_namespace:ident) {
                into_datum: {
                    $( $into_datum_others_pat:pat => $into_datum_others_expr: expr, )*
                    $(,)?
                };
            }
        ) => {
            paste::paste! {
                /// Represents a generic Data type.
                #[must_use]
                #[derive(Debug, strum::EnumDiscriminants, strum::VariantNames)]
                #[strum_discriminants(name(DatumStrType))]
                #[strum_discriminants(derive(Deserialize))]
                pub enum DatumStr {
                    $(
                                     $id_name($crate::id::deserialize::StrId),
                             [<$id_name Set>](::std::vec::Vec<$crate::id::deserialize::StrId>),
                          [<$id_name SetMap>](::std::vec::Vec<($crate::id::deserialize::StrId, ::std::vec::Vec<$crate::id::deserialize::StrId>)>),
                            [<$id_name Grid>](::std::vec::Vec<($crate::game::coord::TileCoord, $crate::id::deserialize::StrId)>),
                          [<$id_name GridXY>](::std::vec::Vec<([$crate::game::coord::TileUnit; 2], $crate::id::deserialize::StrId)>),
                    )*
                    $(
                        $name($type),
                    )*
                }

                impl DatumStr {
                    pub fn into_datum(
                        self,
                        $interner: &mut IdInterner,
                        $fallback_namespace: Option<&str>,
                    ) -> Result<Datum, StrIdParseError> {
                        use $crate::id::parse::{parse_ids, parse_map_item_id};

                        Ok(match self {
                            $(
                                DatumStr::$id_name(v) => Datum::$id_name($interner.get_or_intern(v, $fallback_namespace)?.into()),
                                DatumStr::[<$id_name Set>](v) => {
                                    Datum::[<Set $id_name>](parse_ids(v.into_iter(), $interner, $fallback_namespace).try_collect()?)
                                },
                                DatumStr::[<$id_name SetMap>](v) => {
                                    let mut r = IdMap::new();

                                    for (id, set) in v {
                                        r.insert(
                                            $interner.get_or_intern(id, $fallback_namespace)?.into(),
                                            parse_ids(set.into_iter(), $interner, $fallback_namespace).try_collect()?,
                                        );
                                    }

                                    Datum::[<MapSet $id_name>](r)
                                },
                                DatumStr::[<$id_name Grid>](v) => {
                                    Datum::[<MapCoord $id_name>](parse_map_item_id(v.into_iter(), $interner, $fallback_namespace).try_collect()?)
                                },
                                DatumStr::[<$id_name GridXY>](v) => {
                                    Datum::[<MapCoord $id_name>](
                                        parse_map_item_id(
                                            v.into_iter()
                                                .map(|([col, row], id)| (TileCoord::from_offset(OffsetCoord::new(col, row)), id)),
                                            $interner,
                                            $fallback_namespace,
                                        )
                                        .try_collect()?,
                                    )
                                },
                            )*
                            $(
                                $into_datum_others_pat => $into_datum_others_expr,
                            )*
                        })
                    }

                    fn parse_serde_enum_variant<'de, A>(variant_name: DatumStrType, variant_data: A::Variant) -> Result<DatumStr, A::Error>
                        where
                            A: ::serde::de::EnumAccess<'de>
                    {
                        use ::serde::de::VariantAccess;

                        match variant_name {
                            $(
                                DatumStrType::$id_name => Ok(DatumStr::$id_name(variant_data.newtype_variant()?)),
                                DatumStrType::[<$id_name Set>] => Ok(DatumStr::[<$id_name Set>](variant_data.newtype_variant()?)),
                                DatumStrType::[<$id_name SetMap>] => Ok(DatumStr::[<$id_name SetMap>](variant_data.newtype_variant()?)),
                                DatumStrType::[<$id_name Grid>] => Ok(DatumStr::[<$id_name Grid>](variant_data.newtype_variant()?)),
                                DatumStrType::[<$id_name GridXY>] => Ok(DatumStr::[<$id_name GridXY>](variant_data.newtype_variant()?)),
                            )*

                            $(
                                DatumStrType::$name => Ok(DatumStr::$name(variant_data.newtype_variant()?)),
                            )*
                        }
                    }
                }
            }
        }
    }

    define_datum_str! {
        ids: [Id, TileId, ItemId, RecipeId, TagId, CategoryId, ResearchId, ScriptId, ModelId, RenderId];
        others: {
            Coord(TileCoord),
            CoordXY([TileUnit; 2]),
            CoordList(Vec<TileCoord>),
            CoordListXY(Vec<[TileUnit; 2]>),
            CoordBounds(TileCoordBounds),

            ItemStack(ItemStackStr),
            Inventory(InventoryStr),

            Color(String),

            Int(Int),
            UInt(UInt),
            Float(Float),
            Bool(bool),
        };

        conv(interner, fallback_namespace) {
            into_datum: {
                DatumStr::Coord(v) => Datum::TileCoord(v),
                DatumStr::CoordXY([col, row]) => Datum::TileCoord(TileCoord::from_offset(OffsetCoord::new(col, row))),
                DatumStr::CoordList(v) => Datum::VecTileCoord(v),
                DatumStr::CoordListXY(v) => Datum::VecTileCoord(
                    v.into_iter()
                        .map(|[col, row]| TileCoord::from_offset(OffsetCoord::new(col, row)))
                        .collect(),
                ),
                DatumStr::CoordBounds(v) => Datum::TileCoordBounds(v),

                DatumStr::ItemStack(v) =>  Datum::ItemStack(ItemStack { id: ItemId(interner.get_or_intern(v.id, fallback_namespace)?), amount: v.amount }),
                DatumStr::Inventory(v) => Datum::Inventory(v.into_inventory(interner, fallback_namespace)?),

                DatumStr::Color(v) => Datum::Color(colors::decode_rgba(&v)),

                DatumStr::Int(v) => Datum::Int(v),
                DatumStr::UInt(v) => Datum::UInt(v),
                DatumStr::Float(v) => Datum::Float(v),
                DatumStr::Bool(v) => Datum::Bool(v),
            };
        }
    }

    mod serde_impl {
        use serde::{
            Deserialize,
            de::{EnumAccess, Error, Expected, Unexpected, Visitor, value::StrDeserializer},
        };
        use strum::VariantNames;

        use crate::game::generic::deserialize::{DatumStr, DatumStrType};

        struct DatumStrVisitor;

        impl<'de> Visitor<'de> for DatumStrVisitor {
            type Value = DatumStr;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("enum `DatumStr`")
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                enum EnumTag {
                    Variant(DatumStrType),
                    Other(String),
                }

                struct EnumTagVisitor;

                impl<'de> Visitor<'de> for EnumTagVisitor {
                    type Value = EnumTag;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("enum tag [")?;
                        let mut count = 0;
                        for variant in DatumStr::VARIANTS {
                            count += 1;
                            formatter.write_str(variant)?;
                            if count != DatumStr::VARIANTS.len() {
                                formatter.write_str(", ")?;
                            }
                        }
                        formatter.write_str("], or an untagged literal")?;

                        Ok(())
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: Error,
                    {
                        match DatumStrType::deserialize(StrDeserializer::<E>::new(v)) {
                            Ok(variant) => Ok(EnumTag::Variant(variant)),
                            Err(_) => Ok(EnumTag::Other(v.to_string())),
                        }
                    }
                }

                impl<'de> Deserialize<'de> for EnumTag {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: serde::Deserializer<'de>,
                    {
                        deserializer.deserialize_identifier(EnumTagVisitor)
                    }
                }

                let (variant_name, variant_data): (EnumTag, _) = data.variant()?;

                match variant_name {
                    EnumTag::Variant(variant_name) => DatumStr::parse_serde_enum_variant::<A>(variant_name, variant_data),
                    EnumTag::Other(v) => {
                        struct ExpectedLiteral;
                        impl Expected for ExpectedLiteral {
                            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                                formatter.write_str("bool or number literal")
                            }
                        }

                        match v.as_str() {
                            // fast-track these because they're simple literals
                            "true" => Ok(DatumStr::Bool(true)),
                            "false" => Ok(DatumStr::Bool(false)),
                            _ => match ron::from_str::<ron::Value>(&v) {
                                Ok(v) => match v {
                                    ron::Value::Bool(v) => Ok(DatumStr::Bool(v)),
                                    ron::Value::Number(number) => Ok(match number {
                                        ron::Number::I8(v) => DatumStr::Int(v as _),
                                        ron::Number::I16(v) => DatumStr::Int(v as _),
                                        ron::Number::I32(v) => DatumStr::Int(v as _),
                                        ron::Number::I64(v) => DatumStr::Int(v as _),
                                        ron::Number::U8(v) => DatumStr::UInt(v as _),
                                        ron::Number::U16(v) => DatumStr::UInt(v as _),
                                        ron::Number::U32(v) => DatumStr::UInt(v as _),
                                        ron::Number::U64(v) => DatumStr::UInt(v as _),
                                        ron::Number::F32(v) => DatumStr::Float(v.0 as _),
                                        ron::Number::F64(v) => DatumStr::Float(v.0 as _),
                                        ron::Number::__NonExhaustive(never) => never.never(),
                                    }),
                                    ron::Value::Char(v) => Err(Error::invalid_value(Unexpected::Char(v), &ExpectedLiteral)),
                                    ron::Value::String(v) => Err(Error::invalid_value(Unexpected::Str(&v), &ExpectedLiteral)),
                                    ron::Value::Bytes(v) => Err(Error::invalid_value(Unexpected::Bytes(&v), &ExpectedLiteral)),
                                    ron::Value::Option(_) => Err(Error::invalid_value(Unexpected::Option, &ExpectedLiteral)),
                                    ron::Value::Seq(_) => Err(Error::invalid_value(Unexpected::Seq, &ExpectedLiteral)),
                                    ron::Value::Map(_) => Err(Error::invalid_value(Unexpected::Map, &ExpectedLiteral)),
                                    ron::Value::Unit => Err(Error::invalid_value(Unexpected::Unit, &ExpectedLiteral)),
                                },
                                Err(e) => Err(Error::custom(e)),
                            },
                        }
                    },
                }
            }
        }

        impl<'de> Deserialize<'de> for DatumStr {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_enum("DatumStr", DatumStr::VARIANTS, DatumStrVisitor)
            }
        }
    }

    #[must_use]
    #[repr(transparent)]
    #[derive(Debug, Default, Deserialize)]
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
        pub fn into_data(self, interner: &mut IdInterner, fallback_namespace: Option<&str>) -> Result<DataMap, StrIdParseError> {
            let mut map = DataMap::new();

            for (id, raw_datum) in self.0.into_iter() {
                map.set(
                    interner.get_or_intern(id, fallback_namespace)?,
                    raw_datum.into_datum(interner, fallback_namespace)?,
                );
            }

            Ok(map)
        }
    }
}
