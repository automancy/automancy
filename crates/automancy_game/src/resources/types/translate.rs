use std::{ffi::OsStr, fmt::Debug, path::Path};

use automancy_data::{
    id::{CategoryId, Id, ItemId, RecipeId, TileId, deserialize::StrId, parse::parse_map_id_static_str},
    id_map::{IdMap, ImmutableIdMap},
};
use hashbrown::HashMap;
use interpolator::Formattable;
use serde::Deserialize;

use crate::{
    format::FormatContext,
    persistent,
    resources::{MutableResourceManager, RON_EXTS, ResourceError, ResourceManager, read_recursively},
};

type Str = &'static str;
type StrRef = Str;

pub struct TranslateDef {
    pub none: Str,
    pub unnamed: Str,

    pub(crate) items: ImmutableIdMap<ItemId, Str>,
    pub(crate) tiles: ImmutableIdMap<TileId, Str>,
    pub(crate) categories: ImmutableIdMap<CategoryId, Str>,
    pub(crate) recipes: ImmutableIdMap<RecipeId, Str>,
    pub(crate) researches: ImmutableIdMap<Id, Str>,

    pub(crate) gui: ImmutableIdMap<Id, Str>,
    pub(crate) error: ImmutableIdMap<Id, Str>,
    pub(crate) keys: ImmutableIdMap<Id, Str>,
}

#[derive(Default)]
pub struct MutableTranslateDef {
    pub(crate) none: Str,
    pub(crate) unnamed: Str,

    pub(crate) items: IdMap<ItemId, Str>,
    pub(crate) tiles: IdMap<TileId, Str>,
    pub(crate) categories: IdMap<CategoryId, Str>,
    pub(crate) recipes: IdMap<RecipeId, Str>,
    pub(crate) researches: IdMap<Id, Str>,

    pub(crate) gui: IdMap<Id, Str>,
    pub(crate) keys: IdMap<Id, Str>,
    pub(crate) error: IdMap<Id, Str>,
}

#[derive(Debug, Deserialize)]
struct Raw {
    #[serde(default)]
    none: Option<String>,
    #[serde(default)]
    unnamed: Option<String>,

    #[serde(default)]
    items: HashMap<StrId, String>,
    #[serde(default)]
    tiles: HashMap<StrId, String>,
    #[serde(default)]
    categories: HashMap<StrId, String>,
    #[serde(default)]
    recipes: HashMap<StrId, String>,

    #[serde(default)]
    gui: HashMap<StrId, String>,
    #[serde(default)]
    error: HashMap<StrId, String>,
    #[serde(default)]
    research: HashMap<StrId, String>,
    #[serde(default)]
    keys: HashMap<StrId, String>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_translate_file(&mut self, file: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading translation definition at: {}.", file.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&std::fs::read_to_string(file)?)?;

        // we leak memory here to make everything else easier :)
        let new = MutableTranslateDef {
            none: v.none.unwrap_or_default().leak(),
            unnamed: v.unnamed.unwrap_or_default().leak(),
            items: { parse_map_id_static_str(v.items.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            tiles: { parse_map_id_static_str(v.tiles.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            categories: { parse_map_id_static_str(v.categories.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            recipes: { parse_map_id_static_str(v.recipes.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            gui: { parse_map_id_static_str(v.gui.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            keys: { parse_map_id_static_str(v.keys.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            error: { parse_map_id_static_str(v.error.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            researches: parse_map_id_static_str(v.research.into_iter(), &mut self.interner, Some(namespace)).try_collect()?,
        };

        if self.translates.none.is_empty() {
            self.translates.none = new.none;
        }
        if self.translates.unnamed.is_empty() {
            self.translates.unnamed = new.unnamed;
        }

        self.translates.items.extend(new.items);
        self.translates.tiles.extend(new.tiles);
        self.translates.categories.extend(new.categories);
        self.translates.recipes.extend(new.recipes);
        self.translates.researches.extend(new.researches);

        self.translates.gui.extend(new.gui);
        self.translates.keys.extend(new.keys);
        self.translates.error.extend(new.error);

        Ok(())
    }

    pub fn load_translate_files(&mut self, dir: &Path, namespace: &str, selected_language: &str) {
        let selected_language = OsStr::new(selected_language);
        let path = dir.join("translates");

        for file in read_recursively(&path, RON_EXTS) {
            if file.file_stem() == Some(selected_language) {
                match self.load_translate_file(&file, namespace) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        }

        log::warn!("Notice: The game leaks memory when loading translation files!");
        log::warn!(
            "Leaking the translation strings allows the game to run more efficiently, but if you're having memory issues, it may be due to this."
        );
        log::warn!("This shouldn't be a problem unless you repeatedly reload the resources without closing the game.");
        log::warn!("If you're a modder and you're reloading the game often, please keep note of this.");
    }
}

macro_rules! impl_translate_utils {
    ($ty:ty) => {
        #[cfg_attr(feature = "profile", profiling::all_functions)]
        impl $ty {
            pub fn item_name(&self, id: ItemId) -> StrRef {
                match self.translates.items.get(&id) {
                    Some(name) => name,
                    None => self.translates.unnamed,
                }
            }

            pub fn try_item_name(&self, id: Option<ItemId>) -> StrRef {
                if let Some(id) = id {
                    self.item_name(id)
                } else {
                    self.translates.none
                }
            }

            pub fn recipe_name(&self, id: RecipeId) -> StrRef {
                match self.translates.recipes.get(&id) {
                    Some(name) => name,
                    None => self.translates.unnamed,
                }
            }

            pub fn try_recipe_name(&self, id: Option<RecipeId>) -> StrRef {
                if let Some(id) = id {
                    self.recipe_name(id)
                } else {
                    self.translates.none
                }
            }

            pub fn tile_name(&self, id: TileId) -> StrRef {
                match self.translates.tiles.get(&id) {
                    Some(name) => name,
                    None => self.translates.unnamed,
                }
            }

            pub fn try_tile_name(&self, id: Option<TileId>) -> StrRef {
                if let Some(id) = id {
                    self.tile_name(id)
                } else {
                    self.translates.none
                }
            }

            pub fn category_name(&self, id: CategoryId) -> StrRef {
                match self.translates.categories.get(&id) {
                    Some(name) => name,
                    None => self.translates.unnamed,
                }
            }

            pub fn try_category_name(&self, id: Option<CategoryId>) -> StrRef {
                if let Some(id) = id {
                    self.category_name(id)
                } else {
                    self.translates.none
                }
            }

            pub fn key_name(&self, id: Id) -> StrRef {
                match self.translates.keys.get(&id) {
                    Some(name) => name,
                    None => self.translates.unnamed,
                }
            }

            pub fn try_key_name(&self, id: Option<Id>) -> StrRef {
                if let Some(id) = id {
                    self.key_name(id)
                } else {
                    self.translates.none
                }
            }

            pub fn gui_str(&self, id: Id) -> StrRef {
                match self.translates.gui.get(&id) {
                    Some(v) => v,
                    None => self.translates.unnamed,
                }
            }

            pub fn gui_fmt<'a, T>(&self, id: Id, fmt: T) -> String
            where
                T: Debug + IntoIterator<Item = (&'a str, Formattable<'a>)>,
            {
                let context = FormatContext::from_iter(fmt);

                match self.translates.gui.get(&id) {
                    Some(v) => context.format_str(v),
                    None => format!("{} - {context}", self.translates.unnamed),
                }
            }

            pub fn research_str(&self, id: Id) -> StrRef {
                match self.translates.researches.get(&id) {
                    Some(v) => v,
                    None => self.translates.unnamed,
                }
            }
        }
    };
}

impl_translate_utils!(MutableResourceManager);
impl_translate_utils!(ResourceManager);
