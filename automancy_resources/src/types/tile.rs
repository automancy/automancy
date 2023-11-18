use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::{Deserialize, Serialize};

use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::data::{DataMap, DataMapRaw};
use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Debug, Deserialize, Serialize)]
pub struct TileRaw {
    pub id: IdRaw,
    pub function: Option<IdRaw>,
    pub models: Vec<IdRaw>,
    #[serde(default)]
    pub data: DataMapRaw,
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub models: Vec<Id>,
    pub function: Option<Id>,
    pub data: DataMap,
}

impl ResourceManager {
    fn load_tile(&mut self, file: &Path) -> anyhow::Result<()> {
        log::info!("Loading tile at {file:?}");

        let tile: TileRaw = ron::from_str(&read_to_string(file)?)?;

        let id = tile.id.to_id(&mut self.interner);

        let function = tile.function.map(|v| v.to_id(&mut self.interner));

        let data = tile.data.intern_to_data(&mut self.interner);

        let models = tile
            .models
            .into_iter()
            .map(|v| v.to_id(&mut self.interner))
            .collect();

        self.registry.tiles.insert(
            id,
            Tile {
                function,
                models,
                data,
            },
        );

        Ok(())
    }
    pub fn load_tiles(&mut self, dir: &Path) -> anyhow::Result<()> {
        let tiles = dir.join("tiles");

        for file in load_recursively(&tiles, OsStr::new(RON_EXT)) {
            self.load_tile(&file)?;
        }

        Ok(())
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
