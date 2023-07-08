use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::Deserialize;
use serde_json;

use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::data::{DataMap, DataMapRaw};
use crate::{load_recursively, ResourceManager, JSON_EXT};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelAttributesJson {
    pub inactive_model: Option<IdRaw>,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelAttributes {
    pub inactive_model: Option<Id>,
}

#[derive(Debug, Deserialize)]
pub struct TileJson {
    pub id: IdRaw,
    pub function: Option<IdRaw>,
    pub models: Vec<IdRaw>,
    #[serde(default)]
    pub data: DataMapRaw,
    #[serde(default)]
    pub model_attributes: ModelAttributesJson,
    pub targeted: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub models: Vec<Id>,
    pub function: Option<Id>,
    pub data: DataMap,
    pub model_attributes: ModelAttributes,
    pub targeted: bool,
}

impl ResourceManager {
    fn load_tile(&mut self, file: &Path) -> Option<()> {
        log::info!("loading tile at {file:?}");

        let tile: TileJson = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
        )
        .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}"));

        let id = tile.id.to_id(&mut self.interner);

        let function = tile.function.map(|v| v.to_id(&mut self.interner));

        let data = tile.data.intern_to_data(self);

        let models = tile
            .models
            .into_iter()
            .map(|v| v.to_id(&mut self.interner))
            .collect();

        let targeted = tile.targeted.unwrap_or(true);

        let model_attributes = ModelAttributes {
            inactive_model: tile
                .model_attributes
                .inactive_model
                .map(|v| v.to_id(&mut self.interner)),
        };

        self.registry.tiles.insert(
            id,
            Tile {
                function,
                model_attributes,
                targeted,
                models,
                data,
            },
        );

        Some(())
    }
    pub fn load_tiles(&mut self, dir: &Path) -> Option<()> {
        let tiles = dir.join("tiles");

        for file in load_recursively(&tiles, OsStr::new(JSON_EXT)) {
            self.load_tile(&file);
        }

        Some(())
    }

    pub fn item_name(&self, id: &Id) -> &str {
        match self.translates.items.get(id) {
            Some(name) => name,
            None => &self.translates.unnamed,
        }
    }

    pub fn try_item_name(&self, id: Option<&Id>) -> &str {
        if let Some(id) = id {
            self.item_name(id)
        } else {
            &self.translates.none
        }
    }

    pub fn script_name(&self, id: &Id) -> &str {
        match self.translates.scripts.get(id) {
            Some(name) => name,
            None => &self.translates.unnamed,
        }
    }

    pub fn try_script_name(&self, id: Option<&Id>) -> &str {
        if let Some(id) = id {
            self.item_name(id)
        } else {
            &self.translates.none
        }
    }

    pub fn tile_name(&self, id: &Id) -> &str {
        match self.translates.tiles.get(id) {
            Some(name) => name,
            None => &self.translates.unnamed,
        }
    }

    pub fn try_tile_name(&self, id: Option<&Id>) -> &str {
        if let Some(id) = id {
            self.tile_name(id)
        } else {
            &self.translates.none
        }
    }
}
