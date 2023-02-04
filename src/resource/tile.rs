use crate::resource::{ResourceManager, JSON_EXT};
use crate::util::id::{Id, IdRaw};
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", content = "param")]
pub enum TileTypeRaw {
    Empty,
    Void,
    Model,
    Machine(IdRaw),
    Transfer(IdRaw),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TileType {
    Empty,
    Void,
    Model,
    Machine(Id),
    Transfer(Id),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileRaw {
    pub tile_type: TileTypeRaw,
    pub id: IdRaw,
    pub scripts: Option<Vec<IdRaw>>,
    pub function: Option<IdRaw>,
    pub models: Vec<IdRaw>,
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub tile_type: TileType,
    pub scripts: Option<Vec<Id>>,
    pub function: Option<Id>,
    pub models: Vec<Id>,
}

impl ResourceManager {
    fn load_tile(&mut self, file: &Path) -> Option<()> {
        log::info!("loading tile at {file:?}");

        let tile: TileRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

        let id = tile.id.to_id(&mut self.interner);

        let scripts = tile.scripts.map(|v| {
            v.into_iter()
                .map(|id| id.to_id(&mut self.interner))
                .collect()
        });

        let tile_type = match tile.tile_type {
            TileTypeRaw::Empty => TileType::Empty,
            TileTypeRaw::Void => TileType::Void,
            TileTypeRaw::Model => TileType::Model,
            TileTypeRaw::Machine(id) => TileType::Machine(id.to_id(&mut self.interner)),
            TileTypeRaw::Transfer(id) => TileType::Transfer(id.to_id(&mut self.interner)),
        };

        let function = tile.function.map(|v| v.to_id(&mut self.interner));

        let models = tile
            .models
            .into_iter()
            .map(|v| v.to_id(&mut self.interner))
            .collect();

        self.tiles.insert(
            id,
            Tile {
                tile_type,
                scripts,
                function,
                models,
            },
        );

        Some(())
    }
    pub fn load_tiles(&mut self, dir: &Path) -> Option<()> {
        let tiles = dir.join("tiles");
        let tiles = read_dir(tiles).ok()?;

        tiles
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .for_each(|tile| {
                self.load_tile(&tile);
            });

        Some(())
    }

    pub fn item_name(&self, id: &Id) -> &str {
        match self.translates.items.get(id) {
            Some(name) => name,
            None => "<unnamed>",
        }
    }

    pub fn try_item_name(&self, id: &Option<Id>) -> &str {
        if let Some(id) = id {
            self.item_name(id)
        } else {
            "<none>"
        }
    }

    pub fn tile_name(&self, id: &Id) -> &str {
        match self.translates.tiles.get(id) {
            Some(name) => name,
            None => "<unnamed>",
        }
    }

    pub fn try_tile_name(&self, id: &Option<Id>) -> &str {
        if let Some(id) = id {
            self.tile_name(id)
        } else {
            "<none>"
        }
    }
}
