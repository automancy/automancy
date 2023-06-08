use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

use serde::Deserialize;

use crate::game::tile::entity::{intern_data_from_raw, DataMap, DataMapRaw};
use crate::resource::{ResourceManager, JSON_EXT};
use crate::util::id::{id_static, Id, IdRaw, Interner};

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct ModelAttributes {
    pub auto_rotate: bool,
}

#[derive(Debug, Deserialize)]
pub struct TileRaw {
    pub id: IdRaw,
    pub tile_type: Option<IdRaw>,
    pub models: Vec<IdRaw>,
    #[serde(default)]
    pub data: DataMapRaw,
    #[serde(default)]
    pub model_attributes: ModelAttributes,
    pub targeted: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub models: Vec<Id>,
    pub tile_type: Option<Id>,
    pub data: DataMap,
    pub model_attributes: ModelAttributes,
    pub targeted: bool,
}

impl ResourceManager {
    fn load_tile(&mut self, file: &Path) -> Option<()> {
        log::info!("loading tile at {file:?}");

        let tile: TileRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
        )
        .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}"));

        let id = tile.id.to_id(&mut self.interner);

        let tile_type = tile.tile_type.map(|v| v.to_id(&mut self.interner));

        let data = intern_data_from_raw(tile.data, &mut self.interner);

        let models = tile
            .models
            .into_iter()
            .map(|v| v.to_id(&mut self.interner))
            .collect();

        let targeted = tile
            .targeted
            .unwrap_or(tile_type == Some(self.registry.tile_ids.machine));

        self.registry.tiles.insert(
            id,
            Tile {
                tile_type,
                model_attributes: tile.model_attributes,
                targeted,
                models,
                data,
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
            None => &self.translates.unnamed,
        }
    }

    pub fn try_item_name(&self, id: &Option<Id>) -> &str {
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

    pub fn try_script_name(&self, id: &Option<Id>) -> &str {
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

    pub fn try_tile_name(&self, id: &Option<Id>) -> &str {
        if let Some(id) = id {
            self.tile_name(id)
        } else {
            &self.translates.none
        }
    }
}

#[derive(Copy, Clone)]
pub struct TileIds {
    pub machine: Id,
    pub transfer: Id,
    pub void: Id,
    pub storage: Id,
    pub merger: Id,
    pub splitter: Id,
    pub master_node: Id,
    pub node: Id,
}

impl TileIds {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            machine: id_static("automancy", "machine").to_id(interner),
            transfer: id_static("automancy", "transfer").to_id(interner),
            void: id_static("automancy", "void").to_id(interner),
            storage: id_static("automancy", "storage").to_id(interner),
            merger: id_static("automancy", "merger").to_id(interner),
            splitter: id_static("automancy", "splitter").to_id(interner),
            master_node: id_static("automancy", "master_node").to_id(interner),
            node: id_static("automancy", "node").to_id(interner),
        }
    }
}

#[derive(Copy, Clone)]
pub struct DataIds {
    pub script: Id,
    pub scripts: Id,
    pub buffer: Id,
    pub storage: Id,
    pub storage_type: Id,
    pub amount: Id,
    pub target: Id,
    pub link: Id,
}

impl DataIds {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            script: id_static("automancy", "script").to_id(interner),
            scripts: id_static("automancy", "scripts").to_id(interner),
            buffer: id_static("automancy", "buffer").to_id(interner),
            storage: id_static("automancy", "storage").to_id(interner),
            storage_type: id_static("automancy", "storage_type").to_id(interner),
            amount: id_static("automancy", "amount").to_id(interner),
            target: id_static("automancy", "target").to_id(interner),
            link: id_static("automancy", "link").to_id(interner),
        }
    }
}
