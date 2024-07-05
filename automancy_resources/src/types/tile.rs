use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::Deserialize;

use automancy_defs::id::Id;

use crate::data::{DataMap, DataMapRaw};
use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Debug, Clone)]
pub struct TileDef {
    pub model: Id,
    pub function: Option<Id>,
    pub data: DataMap,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: String,
    pub function: Option<String>,
    pub model: String,
    pub data: DataMapRaw,
}

impl ResourceManager {
    fn load_tile(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading tile at {file:?}");

        let v = ron::from_str::<Raw>(&read_to_string(file)?)?;

        let id = Id::parse(&v.id, &mut self.interner, Some(namespace)).unwrap();
        let function = v
            .function
            .map(|v| Id::parse(&v, &mut self.interner, Some(namespace)).unwrap());
        let model = Id::parse(&v.model, &mut self.interner, Some(namespace)).unwrap();

        let data = v.data.intern_to_data(&mut self.interner, Some(namespace));

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

    pub fn load_tiles(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let tiles = dir.join("tiles");

        for file in load_recursively(&tiles, OsStr::new(RON_EXT)) {
            self.load_tile(&file, namespace)?;
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
