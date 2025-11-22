use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_data::{
    game::generic::{DataMap, deserialize::DataMapStr},
    id::{
        Id, TileId,
        deserialize::{StrId, StrIdExt},
    },
};
use serde::Deserialize;

use crate::{
    persistent,
    resources::{RON_EXT, ResourceManager, load_recursively},
};

#[derive(Debug, Clone)]
pub struct TileDef {
    pub id: TileId,
    pub script: Option<Id>,
    pub category: Option<Id>,
    pub data: DataMap,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: StrId,
    #[serde(default)]
    pub script: Option<StrId>,
    #[serde(default)]
    pub category: Option<StrId>,
    pub data: DataMapStr,
}

impl ResourceManager {
    fn load_tile(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading tile at {file:?}");

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = TileId(v.id.into_id(&mut self.interner, Some(namespace))?);
        let script = v.script.into_id(&mut self.interner, Some(namespace))?;
        let category = v.category.into_id(&mut self.interner, Some(namespace))?;
        let data = v.data.into_data(&mut self.interner, Some(namespace))?;

        self.registry.tile_defs.insert(id, TileDef { id, script, category, data });

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
        let mut ids = self.registry.tile_defs.keys().cloned().collect::<Vec<_>>();

        ids.sort_by_key(|id| self.tile_name(*id));

        if let Some(none_index) = ids
            .iter()
            .enumerate()
            .find_map(|(index, id)| if **id == self.registry.none { Some(index) } else { None })
        {
            let old = ids.remove(none_index);
            ids.insert(0, old);
        }

        self.ordered_tiles = ids;
    }
}
