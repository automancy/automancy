use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::{Deserialize, Serialize};

use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::data::{DataMap, DataMapRaw};
use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Debug, Deserialize, Serialize)]
pub struct TileDefRaw {
    pub id: IdRaw,
    pub function: Option<IdRaw>,
    pub model: IdRaw,
    #[serde(default)]
    pub data: DataMapRaw,
}

#[derive(Debug, Clone)]
pub struct TileDef {
    pub model: Id,
    pub function: Option<Id>,
    pub data: DataMap,
}

impl ResourceManager {
    fn load_tile(&mut self, file: &Path) -> anyhow::Result<()> {
        log::info!("Loading tile at {file:?}");

        let tile: TileDefRaw = ron::from_str(&read_to_string(file)?)?;

        let id = tile.id.to_id(&mut self.interner);
        let function = tile.function.map(|v| v.to_id(&mut self.interner));
        let data = tile.data.intern_to_data(&mut self.interner);
        let model = tile.model.to_id(&mut self.interner);

        self.registry.tiles.insert(
            id,
            TileDef {
                function,
                model,
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

    pub fn ordered_tiles(&mut self) {
        let mut ids = self.registry.tiles.keys().cloned().collect::<Vec<_>>();

        ids.sort_by_key(|id| self.tile_name(id));

        if let Some(none_idx) = ids.iter().enumerate().find_map(|(idx, id)| {
            if *id == self.registry.none {
                Some(idx)
            } else {
                None
            }
        }) {
            let old = ids.remove(none_idx);
            ids.insert(0, old);
        }

        self.ordered_tiles = ids;
    }
}
