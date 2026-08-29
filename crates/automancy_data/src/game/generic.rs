use core::fmt::{Debug, Display};
use std::collections::BTreeMap;

use bytemuck::TransparentWrapper;

use crate::{
    game::{coord::*, inventory::*},
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
                pub fn $fn_name(&self, id: Id) -> Option<&$datum_type> {
                    self.$field_name.get(&id)
                }

                #[inline]
                pub fn [<$fn_name _mut>](&mut self, id: Id) -> Option<&mut $datum_type> {
                    self.$field_name.get_mut(&id)
                }

                #[inline]
                pub fn [<remove_ $fn_name>](&mut self, id: Id) {
                    self.$field_name.remove(&id);
                }

                #[inline]
                pub fn [<insert_ $fn_name>](&mut self, id: Id, value: $datum_type) {
                    self.$field_name.insert(id, value);
                }

                #[inline]
                pub fn [<$fn_name _then>]<R>(&self, id: Id, f: impl FnOnce(&$datum_type) -> R) -> Option<R> {
                    if let Some(v) = self.$fn_name(id) {
                        Some(f(v))
                    } else {
                        None
                    }
                }

                #[inline]
                pub fn [<$fn_name _mut_or>](&mut self, id: Id, default: $datum_type) -> &mut $datum_type {
                    if !self.$field_name.contains_key(&id) {
                        self.$field_name.insert(id, default);
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
                    self.$field_name.get(&id).map(TransparentWrapper::wrap_ref)
                }

                #[inline]
                pub fn [<$fn_name _mut>](&mut self, id: Id) -> Option<&mut $datum_type> {
                    self.$field_name.get_mut(&id).map(TransparentWrapper::wrap_mut)
                }

                #[inline]
                pub fn [<remove_ $fn_name>](&mut self, id: Id) {
                    self.$field_name.remove(&id);
                }

                #[inline]
                pub fn [<insert_ $fn_name>](&mut self, id: Id, value: $datum_type) {
                    self.$field_name.insert(id, TransparentWrapper::peel(value));
                }

                #[inline]
                pub fn [<$fn_name _then>]<R>(&self, id: Id, f: impl FnOnce(&$datum_type) -> R) -> Option<R> {
                    if let Some(v) = self.$fn_name(id) {
                        Some(f(v))
                    } else {
                        None
                    }
                }

                #[inline]
                pub fn [<$fn_name _mut_or>](&mut self, id: Id, default: $datum_type) -> &mut $datum_type {
                    if !self.$field_name.contains_key(&id) {
                        self.$field_name.insert(id, TransparentWrapper::peel(default));
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

macro_rules! impl_data_of_collections_ids {
    ($data_type:ty {
        $(
            $fn_name:ident: {
                $datum_name_single:ident($datum_type_single:ty),
                $datum_name_set:ident($datum_type_set:ty),
            }
        ),*
        $(,)?
    }) => {
        $(paste::paste! {
            impl $data_type {
                pub fn [<contains_ $fn_name>](&self, key: Id, v: $datum_type_single) -> bool {
                    if let Some(r) = self.$datum_name_single(key) {
                        *r == v
                    } else if let Some(r) = self.$datum_name_set(key) {
                        r.contains(&v)
                    } else {
                        false
                    }
                }

                pub fn [<$fn_name s>](&self, key: Id) -> $datum_type_set {
                    if let Some(r) = self.$datum_name_single(key) {
                        FromIterator::from_iter([*r])
                    } else if let Some(r) = self.$datum_name_set(key) {
                        r.clone()
                    } else {
                        Default::default()
                    }
                }

                pub fn [<add_ $fn_name>](&mut self, key: Id, v: $datum_type_single) {
                    if let Some(r) = self.$datum_name_single(key) {
                        let r = *r;
                        self.[<remove_ $datum_name_single>](key);
                        self.[<insert_ $datum_name_set>](key, FromIterator::from_iter([r, v]));
                    } else if let Some(r) = self.[<$datum_name_set _mut>](key) {
                        r.push(v);
                    } else {
                        self.[<insert_ $datum_name_single>](key, v);
                    }
                }

                pub fn [<sub_ $fn_name>](&mut self, key: Id, v: $datum_type_single) {
                    if let Some(r) = self.$datum_name_single(key) {
                        if (*r == v) {
                            self.[<remove_ $datum_name_single>](key);
                        }
                    } else if let Some(r) = self.[<$datum_name_set _mut>](key) {
                        r.remove(&v);
                    } else {
                        self.[<insert_ $datum_name_single>](key, v);
                    }
                }
            }
        })+
    }
}

macro_rules! impl_data_of_collections_others {
    ($data_type:ty {
        $(
            $fn_name:ident: {
                $datum_name_single:ident($datum_type_single:ty),
                $datum_name_set:ident($datum_type_set:ty),
            }
        ),*
        $(,)?
    }) => {
        $(paste::paste! {
            impl $data_type {
                pub fn [<contains_ $fn_name>](&self, key: Id, v: $datum_type_single) -> bool {
                    if let Some(r) = self.$datum_name_single(key) {
                        *r == v
                    } else if let Some(r) = self.$datum_name_set(key) {
                        r.contains(&v)
                    } else {
                        false
                    }
                }

                pub fn [<$fn_name s>](&self, key: Id) -> $datum_type_set {
                    if let Some(r) = self.$datum_name_single(key) {
                        FromIterator::from_iter([*r])
                    } else if let Some(r) = self.$datum_name_set(key) {
                        r.clone()
                    } else {
                        Default::default()
                    }
                }

                pub fn [<add_ $fn_name>](&mut self, key: Id, v: $datum_type_single) {
                    if let Some(r) = self.[<$datum_name_single _mut>](key) {
                        *r += v;
                    } else if let Some(r) = self.[<$datum_name_set _mut>](key) {
                        r.push(v);
                    } else {
                        self.[<insert_ $datum_name_single>](key, v);
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
    ) => {
        paste::paste! {
            /// Represents a generic Data type.
            #[must_use]
            #[derive(Debug, Clone, strum::EnumDiscriminants, strum::VariantNames)]
            #[strum_discriminants(name(DatumType))]
            pub enum Datum {
                $(
                               $id_name  ($id_name),
                         [<Set $id_name>](IdSet<$id_name>),
                      [<MapSet $id_name>](IdMap<$id_name, IdSet<$id_name>>),
                    [<MapCoord $id_name>](IdCoordMap<$id_name>),
                )*
                $(
                    $name($type),
                )*
            }

            impl DataMap {
                pub fn handle_change(&mut $handle_change_self, change: DatumChange) {
                    match change {
                        DatumChange::Add($handle_change_key, datum) => match datum {
                            $(
                                Datum::$id_name(v) => {
                                    $handle_change_self.[<add_ $id_name:snake>]($handle_change_key, v);
                                },
                                Datum::[<Set $id_name>](v) => {
                                    $handle_change_self.[<set_ $id_name:snake _mut_or_default>]($handle_change_key).extend(v);
                                },
                                Datum::[<MapSet $id_name>](v) => {
                                    let id_map = $handle_change_self.[<map_set_ $id_name:snake _mut_or_default>]($handle_change_key);
                                    for (id, set) in v {
                                        id_map.entry(id).or_default().extend(set);
                                    }
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
                                Datum::$id_name(v) => {
                                    $handle_change_self.[<sub_ $id_name:snake>]($handle_change_key, v);
                                },
                                Datum::[<Set $id_name>](v) => {
                                    let set = $handle_change_self.[<set_ $id_name:snake _mut_or_default>]($handle_change_key);
                                    for id in v {
                                        set.remove(&id);
                                    }
                                },
                                Datum::[<MapSet $id_name>](v) => {
                                    let id_map = $handle_change_self.[<map_set_ $id_name:snake _mut_or_default>]($handle_change_key);
                                    for (id, set) in v {
                                        for v in set {
                                            id_map.entry(id).or_default().remove(&v);
                                        }
                                    }
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
                                Datum::[<Set $id_name>](v) => {
                                    $handle_change_self.[<insert_set_ $id_name:snake>]($handle_change_key, v);
                                },
                                Datum::[<MapSet $id_name>](v) => {
                                    $handle_change_self.[<insert_map_set_ $id_name:snake>]($handle_change_key, v);
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
                                DatumType::[<Set $id_name>] => {
                                    $handle_change_self.[<remove_set_ $id_name:snake>]($handle_change_key);
                                },
                                DatumType::[<MapSet $id_name>] => {
                                    $handle_change_self.[<remove_map_set_ $id_name:snake>]($handle_change_key);
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
                        id_set @            [<set_ $id_name:snake>]: IdSet<$id_name>,
                        id_sets @       [<map_set_ $id_name:snake>]: IdMap<$id_name, IdSet<$id_name>>,
                        coord_to_id @ [<map_coord_ $id_name:snake>]: IdCoordMap<$id_name>,
                    )*
                }
            }

            impl_data_getters_others! {
                DataMap {
                    $( $field_name @ $field_name: $type ),*
                }
            }

            impl_data_of_collections_ids! {
                DataMap {
                    $(
                        [<$id_name:snake>]: {
                                 [<$id_name:snake>]($id_name),
                            [<set_ $id_name:snake>](IdSet<$id_name>),
                        },
                    )*
                }
            }

            impl_data_of_collections_others! {
                DataMap {
                    coord: {
                        coord(TileCoord),
                        vec_coord(Vec<TileCoord>),
                    },

                    color: {
                        color(Rgba),
                        vec_color(Vec<Rgba>),
                    },
                    int: {
                        int(Int),
                        vec_int(Vec<Int>),
                    },
                    uint: {
                        uint(UInt),
                        vec_uint(Vec<UInt>),
                    },
                    float: {
                        float(Float),
                        vec_float(Vec<Float>),
                    },
                }
            }

            impl IntoIterator for DataMap {
                type Item = (Id, Datum);
                type IntoIter = impl Iterator<Item = (Id, Datum)>;

                fn into_iter(self) -> Self::IntoIter {
                    self.id.into_iter().map(|(id, v)| (id, Datum::Id(v)))
                        .chain(self.id_set.into_iter().map(|(id, v)| (id, Datum::SetId(v))))
                        .chain(self.id_sets.into_iter().map(|(id, v)| (id, Datum::MapSetId(v))))
                        .chain(self.coord_to_id.into_iter().map(|(id, v)| (id, Datum::MapCoordId(v))))
                    $(
                        .chain(self.$field_name.into_iter().map(|(id, v)| (id, Datum::$name(v))))
                    )*
                }
            }

            impl DataMap {
                pub fn into_values(self) -> impl Iterator<Item = Datum> {
                    self.id.into_values().map(Datum::Id)
                        .chain(self.id_set.into_values().map(Datum::SetId))
                        .chain(self.id_sets.into_values().map(Datum::MapSetId))
                        .chain(self.coord_to_id.into_values().map(Datum::MapCoordId))
                    $(
                        .chain(self.$field_name.into_values().map(Datum::$name))
                    )*
                }

                pub fn keys(&self) -> impl Iterator<Item = Id> {
                    self.id.keys().copied()
                        .chain(self.id_set.keys().copied())
                        .chain(self.id_sets.keys().copied())
                        .chain(self.coord_to_id.keys().copied())
                    $(
                        .chain(self.$field_name.keys().copied())
                    )*
                }

                #[deprecated(note="use individual methods instead.")]
                pub fn get(&self, id: Id) -> Option<Datum> {
                    self.id.get(&id).cloned().map(Datum::Id)
                        .or_else(|| self.id_set.get(&id).cloned().map(Datum::SetId))
                        .or_else(|| self.id_sets.get(&id).cloned().map(Datum::MapSetId))
                        .or_else(|| self.coord_to_id.get(&id).cloned().map(Datum::MapCoordId))
                    $(
                        .or_else(|| self.$field_name.get(&id).cloned().map(Datum::$name))
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
        vec_coord: VecTileCoord(Vec<TileCoord>),
        coord_bounds: TileCoordBounds(TileCoordBounds),

        item: ItemStack(ItemStack),
        inventory: Inventory(Inventory),

        color: Color(Rgba),
        vec_color: VecColor(Vec<Rgba>),

        int: Int(Int),
        vec_int: VecInt(Vec<Int>),
        uint: UInt(UInt),
        vec_uint: VecUInt(Vec<UInt>),
        float: Float(Float),
        vec_float: VecFloat(Vec<Float>),
        bool: Bool(bool)
    };

    handle_change(self, key) {
        add: {
            datum @ (
                  Datum::TileCoordBounds(..)
                | Datum::Bool(..)
            ) => {
                panic!("cannot add {datum:?} to `DataMap`");
            },

            Datum::TileCoord(v) => {
                self.add_coord(key, v);
            },
            Datum::VecTileCoord(v) => {
                for v in v.into_iter() {
                    self.add_coord(key, v);
                }
            },

            Datum::ItemStack(v) => {
                self.add_stack(key, v);
            },
            Datum::Inventory(v) => {
                for v in v.into_stacks() {
                    self.add_stack(key, v);
                }
            },

            Datum::Color(v) => {
                self.add_color(key, v);
            },
            Datum::VecColor(v) => {
                for v in v.into_iter() {
                    self.add_color(key, v);
                }
            },

            Datum::Int(v) => {
                self.add_int(key, v);
            },
            Datum::VecInt(v) => {
                for v in v.into_iter() {
                    self.add_int(key, v);
                }
            },
            Datum::UInt(v) => {
                self.add_uint(key, v);
            },
            Datum::VecUInt(v) => {
                for v in v.into_iter() {
                    self.add_uint(key, v);
                }
            },
            Datum::Float(v) => {
                self.add_float(key, v)
            },
            Datum::VecFloat(v) => {
                for v in v.into_iter() {
                    self.add_float(key, v);
                }
            },
        };

        sub: {
            datum @ (
                  Datum::TileCoordBounds(..)
                | Datum::Color(..)
                | Datum::VecColor(..)
                | Datum::VecInt(..)
                | Datum::VecUInt(..)
                | Datum::VecFloat(..)
                | Datum::Bool(..)
            ) => {
                panic!("cannot subtract {datum:?} from `DataMap`");
            },

            Datum::TileCoord(v) => {
                self.add_coord(key, -v);
            },
            Datum::VecTileCoord(v) => {
                for v in v.into_iter() {
                    self.add_coord(key, -v);
                }
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
                self.add_int(key, -v);
            },
            Datum::UInt(v) => {
                let uint = self.uint_mut_or_default(key);
                *uint = uint.saturating_sub(v);
            },
            Datum::Float(v) => {
                self.add_float(key, -v);
            },
        };
    };
}

#[derive(Debug, Clone)]
pub enum DatumChange {
    Add(Id, Datum),
    Sub(Id, Datum),
    Set(Id, Datum),
    Remove(Id, DatumType),
}

#[must_use]
#[derive(Debug, Clone, Default)]
pub struct DataMap {
    id: IdMap<Id, Id>,
    id_set: IdMap<Id, IdSet<Id>>,
    id_sets: IdMap<Id, IdMap<Id, IdSet<Id>>>,
    coord_to_id: IdMap<Id, IdCoordMap<Id>>,

    coord: IdMap<Id, TileCoord>,
    vec_coord: IdMap<Id, Vec<TileCoord>>,
    coord_bounds: IdMap<Id, TileCoordBounds>,

    item: IdMap<Id, ItemStack>,
    inventory: IdMap<Id, Inventory>,

    color: IdMap<Id, Rgba>,
    vec_color: IdMap<Id, Vec<Rgba>>,

    int: IdMap<Id, Int>,
    vec_int: IdMap<Id, Vec<Int>>,
    uint: IdMap<Id, UInt>,
    vec_uint: IdMap<Id, Vec<UInt>>,
    float: IdMap<Id, Float>,
    vec_float: IdMap<Id, Vec<Float>>,
    bool: IdMap<Id, bool>,
}

impl DataMap {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.id.clear();
        self.id_set.clear();
        self.id_sets.clear();
        self.coord_to_id.clear();

        self.coord.clear();
        self.vec_coord.clear();
        self.coord_bounds.clear();

        self.item.clear();
        self.inventory.clear();

        self.color.clear();
        self.vec_color.clear();

        self.int.clear();
        self.vec_int.clear();
        self.uint.clear();
        self.vec_uint.clear();
        self.float.clear();
        self.vec_float.clear();
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

    pub fn contains_stack(&self, key: Id, v: ItemStack) -> bool {
        if let Some(r) = self.item(key) {
            r.id == v.id && r.amount >= v.amount
        } else if let Some(r) = self.inventory(key) {
            r.contains(v)
        } else {
            false
        }
    }
    pub fn stacks(&self, key: Id) -> Inventory {
        if let Some(r) = self.item(key) {
            FromIterator::from_iter([*r])
        } else if let Some(r) = self.inventory(key) {
            r.clone()
        } else {
            Default::default()
        }
    }
    pub fn add_stack(&mut self, key: Id, v: ItemStack) {
        if let Some(r) = self.item_mut(key) {
            if r.id == v.id {
                r.amount += v.amount;
            } else {
                let r = *r;

                self.remove_item(key);
                self.insert_inventory(key, Inventory::from_iter([r, v]));
            }
        } else if let Some(r) = self.inventory_mut(key) {
            r.add(v.id, v.amount);
        } else {
            self.insert_item(key, v);
        }
    }
    pub fn remove_stack(&mut self, key: Id, v: ItemStack) {
        if let Some(r) = self.item_mut(key) {
            if r.id == v.id {
                r.amount -= v.amount;
                if r.amount == 0 {
                    self.remove_item(key);
                }
            } else {
                let r = *r;

                self.remove_item(key);
                self.insert_inventory(key, Inventory::from_iter([r, -v]));
            }
        } else if let Some(r) = self.inventory_mut(key) {
            r.add(v.id, -v.amount);
            if r.get(v.id) == 0 {
                r.remove(&v.id);
            }
        } else {
            self.insert_item(key, -v);
        }
    }
}

impl Display for DataMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;
        f.write_fmt(format_args!("\n\tid: {},", self.id))?;
        f.write_fmt(format_args!("\n\tid_set: {},", self.id_set))?;
        f.write_fmt(format_args!("\n\tid_sets: {},", self.id_sets))?;
        f.write_fmt(format_args!("\n\tcoord_to_id: {},", self.coord_to_id))?;
        f.write_fmt(format_args!("\n\tcoord: {},", self.coord))?;
        f.write_fmt(format_args!("\n\tvec_coord: {:?},", self.vec_coord))?;
        f.write_fmt(format_args!("\n\tcoord_bounds: {},", self.coord_bounds))?;
        f.write_fmt(format_args!("\n\titem: {},", self.item))?;
        f.write_fmt(format_args!("\n\tinventory: {},", self.inventory))?;
        f.write_fmt(format_args!("\n\tcolor: {},", self.color))?;
        f.write_fmt(format_args!("\n\tvec_color: {:?},", self.vec_color))?;
        f.write_fmt(format_args!("\n\tint: {},", self.int))?;
        f.write_fmt(format_args!("\n\tvec_int: {:?},", self.vec_int))?;
        f.write_fmt(format_args!("\n\tuint: {},", self.uint))?;
        f.write_fmt(format_args!("\n\tvec_uint: {:?},", self.vec_uint))?;
        f.write_fmt(format_args!("\n\tfloat: {},", self.float))?;
        f.write_fmt(format_args!("\n\tvec_float: {:?},", self.vec_float))?;
        f.write_fmt(format_args!("\n\tbool: {},", self.bool))?;
        f.write_str("\n}")?;

        Ok(())
    }
}

pub mod serailize {
    use core::ops::{Deref, DerefMut};
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
                                    $id_name ($id_name),
                             [<Set $id_name>](Vec<$id_name>),
                          [<MapSet $id_name>](Vec<(Id, Vec<$id_name>)>),
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
            VecColor(Vec<String>),

            Int(Int),
            VecInt(Vec<Int>),
            UInt(UInt),
            VecUInt(Vec<UInt>),
            Float(Float),
            VecFloat(Vec<Float>),
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
                DatumRaw::VecColor(v) => Datum::VecColor(v.iter().map(|v| colors::decode_rgba(v)).collect()),

                DatumRaw::Int(v) => Datum::Int(v),
                DatumRaw::VecInt(v) => Datum::VecInt(v),
                DatumRaw::UInt(v) => Datum::UInt(v),
                DatumRaw::VecUInt(v) => Datum::VecUInt(v),
                DatumRaw::Float(v) => Datum::Float(v),
                DatumRaw::VecFloat(v) => Datum::VecFloat(v),
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
                Datum::VecColor(v) => DatumRaw::VecColor(v.iter().map(|v| colors::encode_rgba(*v)).collect()),

                Datum::Int(v) => DatumRaw::Int(v),
                Datum::VecInt(v) => DatumRaw::VecInt(v),
                Datum::UInt(v) => DatumRaw::UInt(v),
                Datum::VecUInt(v) => DatumRaw::VecUInt(v),
                Datum::Float(v) => DatumRaw::Float(v),
                Datum::VecFloat(v) => DatumRaw::VecFloat(v),
                Datum::Bool(v) => DatumRaw::Bool(v),
            };
        }
    }

    #[must_use]
    #[repr(transparent)]
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct DataMapRaw(BTreeMap<Id, DatumRaw>);

    const impl Deref for DataMapRaw {
        type Target = BTreeMap<Id, DatumRaw>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    const impl DerefMut for DataMapRaw {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    const impl From<BTreeMap<Id, DatumRaw>> for DataMapRaw {
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
    use core::ops::{Deref, DerefMut};

    use serde::Deserialize;

    use super::*;
    use crate::{
        game::inventory::deserialize::{InventoryStr, ItemStackStr},
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
                             [<$id_name Set>](Vec<StrId>),
                          [<$id_name SetMap>](Vec<(StrId, Vec<StrId>)>),
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
                        use $crate::id::parse::{parse_ids, parse_map_item_id};

                        Ok(match self {
                            $(
                                DatumStr::$id_name(v) => Datum::$id_name($interner.get_or_intern(&v, $fallback_namespace)?.into()),
                                DatumStr::[<$id_name Set>](v) => {
                                    Datum::[<Set $id_name>](parse_ids(v.into_iter(), $interner, $fallback_namespace).try_collect()?)
                                },
                                DatumStr::[<$id_name SetMap>](v) => {
                                    let mut r = IdMap::new();

                                    for (id, set) in v {
                                        r.insert(
                                            $interner.get_or_intern(&id, $fallback_namespace)?.into(),
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

                DatumStr::ItemStack(v) =>  Datum::ItemStack(ItemStack { id: ItemId(interner.get_or_intern(&v.id, fallback_namespace)?), amount: v.amount }),
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

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
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

    const impl Deref for DataMapStr {
        type Target = BTreeMap<StrId, DatumStr>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    const impl DerefMut for DataMapStr {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    const impl From<BTreeMap<StrId, DatumStr>> for DataMapStr {
        fn from(value: BTreeMap<StrId, DatumStr>) -> Self {
            Self(value)
        }
    }

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
