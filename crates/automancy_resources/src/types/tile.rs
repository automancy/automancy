use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_defs::id::{Id, TileId};
use serde::Deserialize;

use crate::{
    RON_EXT, ResourceManager,
    data::{DataMap, DataMapRaw},
    load_recursively,
};

#[derive(Debug, Clone)]
pub struct TileDef {
    pub id: TileId,
    pub function: Option<Id>,
    pub category: Option<Id>,
    pub data: DataMap,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: String,
    pub function: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    pub data: DataMapRaw,
}

impl ResourceManager {
    fn load_tile(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading tile at {file:?}");

        let v = ron::from_str::<Raw>(&read_to_string(file)?)?;

        let id = TileId(Id::parse(&v.id, &mut self.interner, Some(namespace)).unwrap());
        let function = v
            .function
            .map(|v| Id::parse(&v, &mut self.interner, Some(namespace)).unwrap());
        let category = v
            .category
            .map(|v| Id::parse(&v, &mut self.interner, Some(namespace)).unwrap());

        let data = v.data.intern_to_data(&mut self.interner, Some(namespace));

        self.registry.tiles.insert(
            id,
            TileDef {
                id,
                function,
                category,
                data,
            },
        );

        Ok(())
    }

    pub fn load_tiles(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let tiles = dir.join("tiles");

        for file in load_recursively(&tiles, OsStr::new(RON_EXT)) {
            self.load_tile(&file, namespace)?;
        }

        Ok(())
    }

    pub fn ordered_tiles(&mut self) {
        let mut ids = self.registry.tiles.keys().cloned().collect::<Vec<_>>();

        ids.sort_by_key(|id| self.tile_name(*id));

        if let Some(none_idx) = ids.iter().enumerate().find_map(|(idx, id)| {
            if **id == self.registry.none {
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
