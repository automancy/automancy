use core::fmt::{Debug, Display};
use std::collections::BTreeMap;

use bytemuck::TransparentWrapper;

use crate::{
    game::{
        coord::*,
        generic::serialize::{DatumRaw, IdMapping},
        inventory::*,
    },
    id::*,
    id_map::*,
    math::*,
    rendering::colors::{self, Rgba},
};

macro_rules! impl_data_getters_others {
    ($data_type:ty {
        $( $field_name:ident @ $fn_name:ident: $datum_type:ty ),*
        $(,)?
    }) => {
        $(paste::paste! {
            impl $data_type {
                #[inline]
                fn [<$fn_name _index>](&mut self, id: Id) -> Option<usize> {
                    self.$field_name.iter().enumerate().find_map(|(idx, (k, _))| if *k == id { Some(idx) } else { None })
                }

                #[inline]
                pub fn $fn_name(&self, id: Id) -> Option<&$datum_type> {
                    self.$field_name.iter().find_map(|(k, v)| if *k == id { Some(v) } else { None })
                }

                #[inline]
                pub fn [<$fn_name _mut>](&mut self, id: Id) -> Option<&mut $datum_type> {
                    self.$field_name.iter_mut().find_map(|(k, v)| if *k == id { Some(v) } else { None })
                }

                #[inline]
                pub fn [<$fn_name _mut_or>](&mut self, id: Id, default: $datum_type) -> &mut $datum_type {
                    if let Some(v) = self.[<$fn_name _mut>](id) {
                        v
                    } else {
                        self.$field_name.push((id, default));
                        let (_, pushed) = self.$field_name.last_mut().unwrap();

                        return pushed;
                    }
                }

                #[inline]
                pub fn [<$fn_name _mut_or_default>](&mut self, id: Id) -> &mut $datum_type
                    where for<'a> $datum_type: Default
                {
                    self.[<$fn_name _mut_or>](id, Default::default())
                }

                #[inline]
                pub fn [<remove_ $fn_name>](&mut self, id: Id) -> Option<$datum_type> {
                    let index = self.[<$fn_name _index>](id)?;
                    let (_, old) = self.$field_name.swap_remove(index);

                    Some(old)
                }

                #[inline]
                pub fn [<insert_ $fn_name>](&mut self, id: Id, value: $datum_type) {
                    if let Some(v) = self.[<$fn_name _mut>](id) {
                        *v = value;
                    } else {
                        self.$field_name.push((id, value));
                    }
                }
            }
        })+
    };
}

macro_rules! impl_data_getters_ids {
    ($data_type:ty {
        $( $field_name:ident @ $fn_name:ident: $datum_type:ty ),*
        $(,)?
    }) => {
        $(paste::paste! {
            impl $data_type {
                #[inline]
                pub fn $fn_name(&self, id: Id) -> Option<&$datum_type> {
                    self.$field_name.iter().find(|(k, _)| *k == id).map(|(_, v)| v).map(TransparentWrapper::wrap_ref)
                }

                #[inline]
                pub fn [<$fn_name _mut>](&mut self, id: Id) -> Option<&mut $datum_type> {
                    self.$field_name.iter_mut().find(|(k, _)| *k == id).map(|(_, v)| v).map(TransparentWrapper::wrap_mut)
                }

                #[inline]
                pub fn [<remove_ $fn_name>](&mut self, id: Id) -> Option<$datum_type> {
                    let index = self.$field_name.iter().enumerate().find(|(_, (k, _))| *k == id).map(|(idx, _)| idx)?;
                    let (_, old) = self.$field_name.swap_remove(index);

                    Some(TransparentWrapper::wrap(old))
                }

                #[inline]
                pub fn [<insert_ $fn_name>](&mut self, id: Id, value: $datum_type) {
                    if let Some((_, v)) = self.$field_name.iter_mut().find(|(k, _)| *k == id) {
                        *v = TransparentWrapper::peel(value);
                    } else {
                        self.$field_name.push((id, TransparentWrapper::peel(value)));
                    }
                }

                #[inline]
                pub fn [<$fn_name _mut_or>](&mut self, id: Id, default: $datum_type) -> &mut $datum_type {
                    if self.$field_name.iter().find(|(k, _)| *k == id).is_none() {
                        self.$field_name.push((id, TransparentWrapper::peel(default)));
                        let (_, pushed) = self.$field_name.last_mut().unwrap();

                        return TransparentWrapper::wrap_mut(pushed);
                    }

                    if let Some(v) = self.[<$fn_name _mut>](id) {
                        v
                    } else {
                        unreachable!();
                    }
                }

                #[inline]
                pub fn [<$fn_name _mut_or_default>](&mut self, id: Id) -> &mut $datum_type
                    where for<'a> $datum_type: Default
                {
                    self.[<$fn_name _mut_or>](id, Default::default())
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
            $( $field_name:ident: $name:ident($type:ty) ),*
            $(,)?
        };

        handle_change($handle_change_self:ident, $handle_change_key:ident) {
            add: {
                $( $handle_add_pat:pat => $handle_add_expr: expr ),*
                $(,)?
            };

            sub: {
                $( $handle_sub_pat:pat => $handle_sub_expr: expr ),*
                $(,)?
            };
        };

        conv($id_map:ident, $interner:ident) {
            into_raw: {
                $( $into_raw_others_pat:pat => $into_raw_others_expr: expr, )*
                $(,)?
            };
        }
    ) => {
        paste::paste! {
            /// Represents a generic Data type.
            #[must_use]
            #[derive(Debug, Clone, strum::EnumDiscriminants, strum::VariantNames)]
            #[strum_discriminants(name(DatumType))]
            pub enum Datum {
                $(
                               $id_name  ($id_name),
                    [<MapCoord $id_name>](IdCoordMap<$id_name>),
                )*
                $(
                    $name($type),
                )*
            }

            impl Datum {
                pub fn into_raw(self, $id_map: &mut IdMapping, $interner: &IdInterner) -> DatumRaw {
                    match self {
                        $(
                            Datum::$id_name(v) => {
                                $id_map.insert(v.into(), $interner);

                                DatumRaw::Id(Id::from(v))
                            },
                            Datum::[<MapCoord $id_name>](v) => {
                                let mut r = Vec::new();

                                for (coord, id) in v {
                                    $id_map.insert(id.into(), $interner);

                                    r.push((coord, Id::from(id)));
                                }

                                DatumRaw::MapCoordId(r)
                            },
                        )*
                        $(
                            $into_raw_others_pat => $into_raw_others_expr,
                        )*
                    }
                }
            }

            impl DataMap {
                pub fn handle_change(&mut $handle_change_self, change: DatumChange) {
                    match change {
                        DatumChange::None => {},
                        DatumChange::Add($handle_change_key, datum) => match datum {
                            $(
                                Datum::$id_name(..) => {
                                    panic!("cannot add {datum:?} to `DataMap`");
                                },
                                Datum::[<MapCoord $id_name>](v) => {
                                    $handle_change_self.[<map_coord_ $id_name:snake _mut_or_default>]($handle_change_key).extend(v);
                                },
                            )*

                            $(
                                $handle_add_pat => $handle_add_expr,
                            )*
                        },
                        DatumChange::Sub($handle_change_key, datum) => match datum {
                            $(
                                Datum::$id_name(..) => {
                                    panic!("cannot subtract {datum:?} from `DataMap`");
                                },
                                Datum::[<MapCoord $id_name>](v) => {
                                    let map = $handle_change_self.[<map_coord_ $id_name:snake _mut_or_default>]($handle_change_key);
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
                        DatumChange::Set($handle_change_key, datum) => match datum {
                            $(
                                Datum::$id_name(v) => {
                                    $handle_change_self.[<insert_ $id_name:snake>]($handle_change_key, v);
                                },
                                Datum::[<MapCoord $id_name>](v) => {
                                    $handle_change_self.[<insert_map_coord_ $id_name:snake>]($handle_change_key, v);
                                },
                            )*
                            $(
                                Datum::$name(v) => {
                                    $handle_change_self.[<insert_ $field_name>]($handle_change_key, v);
                                }
                            )*
                        },
                        DatumChange::Remove($handle_change_key, ty) => match ty {
                            $(
                                DatumType::$id_name => {
                                    $handle_change_self.[<remove_ $id_name:snake>]($handle_change_key);
                                },
                                DatumType::[<MapCoord $id_name>] => {
                                    $handle_change_self.[<remove_map_coord_ $id_name:snake>]($handle_change_key);
                                },
                            )*
                            $(
                                DatumType::$name => {
                                    $handle_change_self.[<remove_ $field_name>]($handle_change_key);
                                }
                            )*
                        },
                    }
                }
            }

            impl_data_getters_ids! {
                DataMap {
                    $(
                        id @                     [<$id_name:snake>]: $id_name,
                        coord_to_id @ [<map_coord_ $id_name:snake>]: IdCoordMap<$id_name>,
                    )*
                }
            }

            impl_data_getters_others! {
                DataMap {
                    $( $field_name @ $field_name: $type ),*
                }
            }

            impl IntoIterator for DataMap {
                type Item = (Id, Datum);
                type IntoIter = impl Iterator<Item = (Id, Datum)>;

                fn into_iter(self) -> Self::IntoIter {
                    self.id.into_iter().map(|(id, v)| (id, Datum::Id(v)))
                        .chain(self.coord_to_id.into_iter().map(|(id, v)| (id, Datum::MapCoordId(v))))
                    $(
                        .chain(self.$field_name.into_iter().map(|(id, v)| (id, Datum::$name(v))))
                    )*
                }
            }

            impl DataMap {
                $(
                    #[inline]
                    pub fn [<contains_ $id_name:snake>](&self, key: Id, id: $id_name) -> bool {
                        self.data(key).map(|data| data.is_true(Id::from(id))).unwrap_or(false)
                    }
                )*

                pub fn into_values(self) -> impl Iterator<Item = Datum> {
                    self.id.into_iter().map(|(_, v)| v).map(Datum::Id)
                        .chain(self.coord_to_id.into_iter().map(|(_, v)| v).map(Datum::MapCoordId))
                    $(
                        .chain(self.$field_name.into_iter().map(|(_, v)| v).map(Datum::$name))
                    )*
                }

                pub fn keys(&self) -> impl Iterator<Item = Id> {
                    self.id.iter().map(|(k, _)| k).copied()
                        .chain(self.coord_to_id.iter().map(|(k, _)| k).copied())
                    $(
                        .chain(self.$field_name.iter().map(|(k, _)| k).copied())
                    )*
                }

                #[deprecated(note="use individual methods instead.")]
                pub fn get(&self, id: Id) -> Option<Datum> {
                    self.id.iter().find(|(k, _)| *k == id).map(|(_, v)| v.clone()).map(Datum::Id)
                        .or_else(|| self.coord_to_id.iter().find(|(k, _)| *k == id).map(|(_, v)| v.clone()).map(Datum::MapCoordId))
                    $(
                        .or_else(|| self.$field_name.iter().find(|(k, _)| *k == id).map(|(_, v)| v.clone()).map(Datum::$name))
                    )*
                }
            }
        }
    }
}

define_datum! {
    ids: [Id, TileId, ItemId, RecipeId, TagId, CategoryId, ResearchId, ScriptId, ModelId, RenderId];
    others: {
        coord: TileCoord(TileCoord),
        coord_bounds: TileCoordBounds(TileCoordBounds),

        items: Items(Inventory),

        color: Color(Rgba),
        int: Int(Int),
        float: Float(Float),
        bool: Bool(bool),

        data: Data(Box<DataMap>),
    };

    handle_change(self, key) {
        add: {
            datum @ (
                  Datum::TileCoordBounds(..)
                | Datum::Color(..)
                | Datum::Bool(..)
            ) => {
                panic!("cannot add {datum:?} to `DataMap`");
            },

            Datum::Items(v) => {
                for v in v.into_stacks() {
                    self.add_stack(key, v);
                }
            },

            Datum::TileCoord(v) => {
                let n = self.coord_mut_or_default(key);
                *n += v;
            },
            Datum::Int(v) => {
                let n = self.int_mut_or_default(key);
                *n = n.saturating_add(v);
            },
            Datum::Float(v) => {
                let n = self.float_mut_or_default(key);
                *n += v;
            },

            Datum::Data(v) => {
                for (k, v) in v.into_iter() {
                    self.handle_change(DatumChange::Add(k, v));
                }
            },
        };

        sub: {
            datum @ (
                  Datum::TileCoordBounds(..)
                | Datum::Color(..)
                | Datum::Bool(..)
            ) => {
                panic!("cannot subtract {datum:?} from `DataMap`");
            },

            Datum::Items(v) => {
                for v in v.into_stacks() {
                    self.remove_stack(key, v);
                }
            },

            Datum::TileCoord(v) => {
                let n = self.coord_mut_or_default(key);
                *n -= v;
            },
            Datum::Int(v) => {
                let n = self.int_mut_or_default(key);
                *n = n.saturating_sub(v);
            },
            Datum::Float(v) => {
                let n = self.float_mut_or_default(key);
                *n -= v;
            },

            Datum::Data(v) => {
                for (k, v) in v.into_iter() {
                    self.handle_change(DatumChange::Sub(k, v));
                }
            },
        };
    };

    conv(id_map, interner) {
        into_raw: {
            Datum::TileCoord(v) => DatumRaw::TileCoord(v),
            Datum::TileCoordBounds(v) => DatumRaw::TileCoordBounds(v),

            Datum::Items(v) => DatumRaw::Items(v.into_raw(id_map, interner)),

            Datum::Color(v) => DatumRaw::Color(colors::encode_rgba(v)),
            Datum::Int(v) => DatumRaw::Int(v),
            Datum::Float(v) => DatumRaw::Float(v),
            Datum::Bool(v) => DatumRaw::Bool(v),

            Datum::Data(v) => DatumRaw::Data(v.into_raw(id_map, interner)),
        };
    }
}

#[derive(Debug, Clone, Default)]
pub enum DatumChange {
    #[default]
    None,
    Add(Id, Datum),
    Sub(Id, Datum),
    Set(Id, Datum),
    Remove(Id, DatumType),
}

#[must_use]
#[derive(Debug, Clone, Default)]
pub struct DataMap {
    id: Vec<(Id, Id)>,
    coord_to_id: Vec<(Id, IdCoordMap<Id>)>,

    coord: Vec<(Id, TileCoord)>,
    coord_bounds: Vec<(Id, TileCoordBounds)>,

    items: Vec<(Id, Inventory)>,

    color: Vec<(Id, Rgba)>,
    int: Vec<(Id, Int)>,
    float: Vec<(Id, Float)>,
    bool: Vec<(Id, bool)>,

    data: Vec<(Id, Box<DataMap>)>,
}

impl DataMap {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.id.clear();
        self.coord_to_id.clear();

        self.coord.clear();
        self.coord_bounds.clear();

        self.items.clear();

        self.color.clear();
        self.int.clear();
        self.float.clear();
        self.bool.clear();
    }

    #[inline]
    pub fn set(&mut self, key: Id, v: Datum) {
        self.handle_change(DatumChange::Set(key, v));
    }

    #[inline]
    pub fn remove(&mut self, key: Id, v: DatumType) {
        self.handle_change(DatumChange::Remove(key, v));
    }

    #[inline]
    pub fn contains_stack(&self, key: Id, v: ItemStack) -> bool {
        if let Some(r) = self.items(key) { r.contains(v) } else { false }
    }
    #[inline]
    pub fn add_stack(&mut self, key: Id, v: ItemStack) {
        if let Some(r) = self.items_mut(key) {
            r.add(v.id, v.amount);
        } else {
            self.insert_items(key, Inventory::from_iter([v]));
        }
    }
    #[inline]
    pub fn remove_stack(&mut self, key: Id, v: ItemStack) {
        if let Some(r) = self.items_mut(key) {
            r.add(v.id, -v.amount);
            if r.get(v.id) == 0 {
                r.remove(&v.id);
            }
        } else {
            self.insert_items(key, Inventory::from_iter([-v]));
        }
    }

    #[inline]
    pub fn is_true<Id: IdLike>(&self, key: Id) -> bool {
        self.bool(key.into()).copied() == Some(true)
    }

    #[inline]
    pub fn is_false<Id: IdLike>(&self, key: Id) -> bool {
        self.bool(key.into()).copied() == Some(false)
    }
}

impl Display for DataMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn format_field<T: Display>(f: &mut std::fmt::Formatter<'_>, v: &[(Id, T)]) -> std::fmt::Result {
            if v.is_empty() {
                return f.write_str("{}, ");
            }
            let last_idx = v.len() - 1;

            f.write_str("{ ")?;
            for (idx, (k, v)) in v.iter().enumerate() {
                f.write_fmt(format_args!("\"{k}\": {v}"))?;
                if idx != last_idx {
                    f.write_str(", ")?;
                }
            }
            f.write_str(" }, ")?;

            Ok(())
        }

        f.write_str("{ ")?;

        {
            format_field(f, &self.id)?;
            format_field(f, &self.coord_to_id)?;

            format_field(f, &self.coord)?;
            format_field(f, &self.coord_bounds)?;

            format_field(f, &self.items)?;

            format_field(f, &self.color)?;
            format_field(f, &self.int)?;
            format_field(f, &self.float)?;
            format_field(f, &self.bool)?;

            format_field(f, &self.data)?;
        }

        f.write_str(" }")?;

        Ok(())
    }
}

pub mod serialize {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};
    use thiserror::Error;

    use super::*;
    use crate::{game::inventory::serialize::InventoryRaw, id::deserialize::StrIdParseError};

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
                .get(id)
                .ok_or_else(|| IdMappingError::InternerMissingStringId(id.to_string()))?;

            Ok(id)
        }

        pub fn get_or_intern(&self, unmapped_id: Id, interner: &mut IdInterner) -> Result<Id, IdMappingError> {
            let id = self.map.get(&unmapped_id).ok_or(IdMappingError::MappingMissingId(unmapped_id))?;
            let id = interner.get_or_intern(id, None)?;

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
            }
        ) => {
            paste::paste! {
                /// Represents a generic Data type.
                #[must_use]
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub enum DatumRaw {
                    $(
                                    $id_name ($id_name),
                        [<MapCoord $id_name>](Vec<(TileCoord, $id_name)>),
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
                                DatumRaw::[<MapCoord $id_name>](v) => {
                                    let mut r = IdCoordMap::new();

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
            }
        }
    }

    define_datum_raw! {
        ids: [Id];
        others: {
            TileCoord(TileCoord),
            TileCoordBounds(TileCoordBounds),

            Items(InventoryRaw),

            Color(String),
            Int(Int),
            Float(Float),
            Bool(bool),

            Data(DataMapRaw),
        };

        conv(id_map, interner) {
            into_datum: {
                DatumRaw::TileCoord(v) => Datum::TileCoord(v),
                DatumRaw::TileCoordBounds(v) => Datum::TileCoordBounds(v),

                DatumRaw::Items(v) => Datum::Items(v.into_inventory(id_map, interner)?),

                DatumRaw::Color(v) => Datum::Color(colors::decode_rgba(&v)),
                DatumRaw::Int(v) => Datum::Int(v),
                DatumRaw::Float(v) => Datum::Float(v),
                DatumRaw::Bool(v) => Datum::Bool(v),

                DatumRaw::Data(v) => Datum::Data(Box::new(v.into_data(id_map, interner)?)),
            };
        }
    }

    #[must_use]
    #[repr(transparent)]
    #[derive(
        Debug,
        Clone,
        Default,
        Serialize,
        Deserialize,
        derive_more::From,
        derive_more::Into,
        derive_more::AsRef,
        derive_more::AsMut,
        derive_more::Deref,
        derive_more::DerefMut,
        derive_more::IntoIterator,
    )]
    #[serde(transparent)]
    pub struct DataMapRaw(BTreeMap<Id, DatumRaw>);

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

            for (id, datum) in self.into_iter() {
                id_map.insert(id, interner);

                if map.contains_key(&id) {
                    log::warn!(
                        "Duplicate Id {id} in `DataMap` when converting to `DataMapRaw`! Do not use duplicate Id's for fields of different types."
                    )
                }

                map.insert(id, datum.into_raw(id_map, interner));
            }

            map
        }
    }
}

pub mod deserialize {
    use serde::Deserialize;

    use super::*;
    use crate::{
        game::inventory::deserialize::InventoryStr,
        id::deserialize::{StrId, StrIdParseError},
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
                                     $id_name(StrId),
                            [<$id_name Grid>](Vec<(TileCoord, StrId)>),
                          [<$id_name GridXY>](Vec<([TileUnit; 2], StrId)>),
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
                        use $crate::id::parse::parse_map_item_id;

                        Ok(match self {
                            $(
                                DatumStr::$id_name(v) => Datum::$id_name($interner.get_or_intern(&v, $fallback_namespace)?.into()),
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
        ids: [Id];
        others: {
            Coord(TileCoord),
            CoordXY([TileUnit; 2]),
            CoordBounds(TileCoordBounds),

            Items(InventoryStr),

            Color(String),
            Int(Int),
            Float(Float),
            Bool(bool),

            Data(DataMapStr),
        };

        conv(interner, fallback_namespace) {
            into_datum: {
                DatumStr::Coord(v) => Datum::TileCoord(v),
                DatumStr::CoordXY([col, row]) => Datum::TileCoord(TileCoord::from_offset(OffsetCoord::new(col, row))),
                DatumStr::CoordBounds(v) => Datum::TileCoordBounds(v),

                DatumStr::Items(v) => Datum::Items(v.into_inventory(interner, fallback_namespace)?),

                DatumStr::Color(v) => Datum::Color(colors::decode_rgba(&v)),
                DatumStr::Int(v) => Datum::Int(v),
                DatumStr::Float(v) => Datum::Float(v),
                DatumStr::Bool(v) => Datum::Bool(v),

                DatumStr::Data(v) => Datum::Data(Box::new(v.into_data(interner, fallback_namespace)?)),
            };
        }
    }

    mod serde_impl {
        use std::borrow::Cow;

        use serde::{
            Deserialize,
            de::{EnumAccess, Error, Visitor, value::StrDeserializer},
        };
        use strum::VariantNames;

        use crate::{
            game::{
                coord::TileUnit,
                generic::deserialize::{DataMapStr, DatumStr, DatumStrType},
            },
            id::deserialize::StrId,
        };

        struct DatumStrVisitor;

        impl<'de> Visitor<'de> for DatumStrVisitor {
            type Value = DatumStr;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("enum `DatumStr`")
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                enum EnumTag {
                    Variant(DatumStrType),
                    ParsedVariant(DatumStr),
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

                    fn visit_map<A>(self, mut v: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::MapAccess<'de>,
                    {
                        #[derive(Debug, Deserialize)]
                        #[serde(untagged)]
                        enum MapKey {
                            Id(String),
                            Coord([TileUnit; 2]),
                        }

                        let mut data_map = DataMapStr::default();
                        let mut id_coord_map = Vec::default();

                        while let Ok(Some(k)) = v.next_key::<MapKey>() {
                            match k {
                                MapKey::Id(k) => {
                                    data_map.insert(Cow::Owned(k), v.next_value::<DatumStr>()?);
                                },
                                MapKey::Coord(coord) => {
                                    id_coord_map.push((coord, v.next_value::<StrId>()?));
                                },
                            }
                        }

                        if !data_map.is_empty() {
                            Ok(EnumTag::ParsedVariant(DatumStr::Data(data_map)))
                        } else if !id_coord_map.is_empty() {
                            Ok(EnumTag::ParsedVariant(DatumStr::IdGridXY(id_coord_map)))
                        } else {
                            Err(A::Error::custom(
                                "expected either a map of string to Datum, or a map of coord to Id",
                            ))
                        }
                    }

                    fn visit_seq<A>(self, mut v: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::SeqAccess<'de>,
                    {
                        let mut map = DataMapStr::default();

                        while let Ok(Some(v)) = v.next_element::<String>() {
                            map.insert(Cow::Owned(v), DatumStr::Bool(true));
                        }

                        Ok(EnumTag::ParsedVariant(DatumStr::Data(map)))
                    }

                    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
                    where
                        E: Error,
                    {
                        Ok(EnumTag::ParsedVariant(DatumStr::Bool(v)))
                    }

                    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
                    where
                        E: Error,
                    {
                        match DatumStrType::deserialize(StrDeserializer::<E>::new(&v)) {
                            Ok(variant) => Ok(EnumTag::Variant(variant)),
                            Err(_) => Ok(EnumTag::ParsedVariant(DatumStr::Id(Cow::Owned(v)))),
                        }
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: Error,
                    {
                        match DatumStrType::deserialize(StrDeserializer::<E>::new(v)) {
                            Ok(variant) => Ok(EnumTag::Variant(variant)),
                            Err(_) => Ok(EnumTag::ParsedVariant(DatumStr::Id(Cow::Owned(v.to_string())))),
                        }
                    }
                }

                impl<'de> Deserialize<'de> for EnumTag {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: serde::Deserializer<'de>,
                    {
                        deserializer.deserialize_any(EnumTagVisitor)
                    }
                }

                let (v, variant_data): (EnumTag, _) = data.variant()?;

                match v {
                    EnumTag::Variant(variant_name) => DatumStr::parse_serde_enum_variant::<A>(variant_name, variant_data),
                    EnumTag::ParsedVariant(v) => Ok(v),
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
    #[derive(
        Debug,
        Default,
        Deserialize,
        derive_more::From,
        derive_more::Into,
        derive_more::AsRef,
        derive_more::AsMut,
        derive_more::Deref,
        derive_more::DerefMut,
        derive_more::IntoIterator,
    )]
    #[serde(transparent)]
    pub struct DataMapStr(BTreeMap<StrId, DatumStr>);

    impl DataMapStr {
        pub fn into_data(self, interner: &mut IdInterner, fallback_namespace: Option<&str>) -> Result<DataMap, StrIdParseError> {
            let mut map = DataMap::new();

            for (id, raw_datum) in self.0.into_iter() {
                map.set(
                    interner.get_or_intern(&id, fallback_namespace)?,
                    raw_datum.into_datum(interner, fallback_namespace)?,
                );
            }

            Ok(map)
        }
    }
}
