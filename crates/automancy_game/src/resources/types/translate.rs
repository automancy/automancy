use std::{
    ffi::OsStr,
    fmt::Debug,
    fs::{read_dir, read_to_string},
    path::Path,
};

use automancy_data::id::{Id, TileId, deserialize::StrId, parse::parse_map_id_item};
use hashbrown::HashMap;
use serde::Deserialize;

use crate::{
    persistent,
    resources::{RON_EXT, ResourceManager},
};

#[derive(Debug, Default)]
pub struct TranslateDef {
    pub none: String,
    pub unnamed: String,

    pub(crate) items: HashMap<Id, String>,
    pub(crate) tiles: HashMap<Id, String>,
    pub(crate) categories: HashMap<Id, String>,
    pub(crate) recipes: HashMap<Id, String>,

    pub(crate) gui: HashMap<Id, String>,
    pub(crate) error: HashMap<Id, String>,
    pub(crate) research: HashMap<Id, String>,
    pub keys: HashMap<Id, String>,
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
    scripts: HashMap<StrId, String>,

    #[serde(default)]
    gui: HashMap<StrId, String>,
    #[serde(default)]
    error: HashMap<StrId, String>,
    #[serde(default)]
    research: HashMap<StrId, String>,
    #[serde(default)]
    keys: HashMap<StrId, String>,
}

impl ResourceManager {
    fn load_translate(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading translate at: {file:?}");

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let new = TranslateDef {
            none: v.none.unwrap_or_default(),
            unnamed: v.unnamed.unwrap_or_default(),
            items: { parse_map_id_item(v.items.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            tiles: { parse_map_id_item(v.tiles.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            categories: { parse_map_id_item(v.categories.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            recipes: { parse_map_id_item(v.scripts.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            gui: { parse_map_id_item(v.gui.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            keys: { parse_map_id_item(v.keys.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            error: { parse_map_id_item(v.error.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
            research: parse_map_id_item(v.research.into_iter(), &mut self.interner, Some(namespace)).try_collect()?,
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
        self.translates.gui.extend(new.gui);
        self.translates.keys.extend(new.keys);
        self.translates.error.extend(new.error);
        self.translates.research.extend(new.research);

        Ok(())
    }

    pub fn load_translates(&mut self, dir: &Path, namespace: &str, selected_language: &str) -> anyhow::Result<()> {
        let lang = OsStr::new(selected_language);

        if let Ok(dir) = read_dir(dir.join("translates")) {
            for file in dir
                .into_iter()
                .flatten()
                .map(|v| v.path())
                .filter(|v| v.extension() == Some(OsStr::new(RON_EXT)))
            {
                if file.file_stem() == Some(lang) {
                    self.load_translate(&file, namespace)?;
                }
            }
        }

        Ok(())
    }

    pub fn item_name(&self, id: Id) -> &str {
        match self.translates.items.get(&id) {
            Some(name) => name.as_str(),
            None => self.translates.unnamed.as_str(),
        }
    }

    pub fn try_item_name(&self, id: Option<Id>) -> &str {
        if let Some(id) = id {
            self.item_name(id)
        } else {
            self.translates.none.as_str()
        }
    }

    pub fn script_name(&self, id: Id) -> &str {
        match self.translates.recipes.get(&id) {
            Some(name) => name.as_str(),
            None => self.translates.unnamed.as_str(),
        }
    }

    pub fn try_script_name(&self, id: Option<Id>) -> &str {
        if let Some(id) = id {
            self.item_name(id)
        } else {
            self.translates.none.as_str()
        }
    }

    pub fn tile_name(&self, id: TileId) -> &str {
        match self.translates.tiles.get(&*id) {
            Some(name) => name.as_str(),
            None => self.translates.unnamed.as_str(),
        }
    }

    pub fn try_tile_name(&self, id: Option<TileId>) -> &str {
        if let Some(id) = id {
            self.tile_name(id)
        } else {
            self.translates.none.as_str()
        }
    }

    pub fn category_name(&self, id: Id) -> &str {
        match self.translates.categories.get(&id) {
            Some(name) => name.as_str(),
            None => self.translates.unnamed.as_str(),
        }
    }

    pub fn try_category_name(&self, id: Option<Id>) -> &str {
        if let Some(id) = id {
            self.category_name(id)
        } else {
            self.translates.none.as_str()
        }
    }

    pub fn gui_str(&self, id: Id) -> &str {
        match self.translates.gui.get(&id) {
            Some(v) => v.as_str(),
            None => self.translates.unnamed.as_str(),
        }
    }

    pub fn research_str(&self, id: Id) -> &str {
        match self.translates.research.get(&id) {
            Some(v) => v.as_str(),
            None => self.translates.unnamed.as_str(),
        }
    }
}
