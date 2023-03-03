use crate::resource::item::{Item, ItemRaw};
use crate::resource::{ResourceManager, JSON_EXT};
use crate::util::id::{id_static, Id, IdRaw, Interner};
use rune::Any;
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
    Machine(Vec<IdRaw>),
    Transfer(IdRaw),
    Storage(ItemRaw),
    Deposit,
}

#[derive(Debug, Clone, PartialEq, Any)]
pub enum TileType {
    Empty,
    Void,
    Model,
    Machine(Vec<Id>),
    Transfer(Id),
    Storage(Item),
    Deposit,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileRaw {
    pub tile_type: TileTypeRaw,
    pub id: IdRaw,
    pub function: Option<IdRaw>,
    pub models: Vec<IdRaw>,
    pub targeted: Option<bool>,
}

#[derive(Debug, Clone, Any)]
pub struct Tile {
    #[rune(get)]
    pub tile_type: TileType,

    pub function: Option<Id>,
    pub models: Vec<Id>,
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

        let tile_type = match tile.tile_type {
            TileTypeRaw::Empty => TileType::Empty,
            TileTypeRaw::Void => TileType::Void,
            TileTypeRaw::Model => TileType::Model,
            TileTypeRaw::Machine(scripts) => TileType::Machine(
                scripts
                    .into_iter()
                    .map(|script| script.to_id(&mut self.interner))
                    .collect(),
            ),
            TileTypeRaw::Transfer(id) => TileType::Transfer(id.to_id(&mut self.interner)),
            TileTypeRaw::Storage(storage) => TileType::Storage(storage.to_item(&mut self.interner)),
            TileTypeRaw::Deposit => TileType::Deposit,
        };

        let function = tile.function.map(|v| v.to_id(&mut self.interner));

        let models = tile
            .models
            .into_iter()
            .map(|v| v.to_id(&mut self.interner))
            .collect();

        let targeted = tile
            .targeted
            .unwrap_or(matches!(&tile_type, TileType::Machine(_)));

        if tile_type == TileType::Deposit {
            self.registry.deposit_tiles.push(id);
        }

        self.registry.tiles.insert(
            id,
            Tile {
                tile_type,
                function,
                models,
                targeted,
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

#[derive(Copy, Clone, Any)]
pub struct TileIds {
    #[rune(get, copy)]
    pub machine: Id,
    #[rune(get, copy)]
    pub master_node: Id,
    #[rune(get, copy)]
    pub node: Id,
}

impl TileIds {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            machine: id_static("automancy", "machine").to_id(interner),
            master_node: id_static("automancy", "master_node").to_id(interner),
            node: id_static("automancy", "node").to_id(interner),
        }
    }
}
