use std::{fs::read_to_string, path::Path};

use automancy_data::{
    game::generic::{DataMap, deserialize::DataMapStr},
    id::{CategoryId, ScriptId, TileId, deserialize::StrId},
};
use serde::Deserialize;

use crate::{
    persistent,
    resources::{MutableResourceManager, RON_EXTS, ResourceError, read_recursively},
};

#[derive(Debug, Clone)]
pub struct TileDef {
    pub id: TileId,
    pub script: ScriptId,
    pub category: CategoryId,
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

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_tile_file(&mut self, file: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading tile definition at {}.", file.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = TileId(self.interner.get_or_intern(v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "tile",
                file: file.to_path_buf(),
                name: self.interner.resolve(*id).unwrap().to_string(),
            });
        }
        let script = ScriptId(self.interner.get_or_intern_opt(v.script, Some(namespace))?);
        let category = CategoryId(self.interner.get_or_intern_opt(v.category, Some(namespace))?);
        let data = v.data.into_data(&mut self.interner, Some(namespace))?;

        self.registry.tile_defs.insert(
            id,
            TileDef {
                id,
                script,
                category,
                data,
            },
        );

        Ok(())
    }

    pub fn load_tile_files(&mut self, dir: &Path, namespace: &str) {
        let path = dir.join("tiles");

        for file in read_recursively(&path, RON_EXTS) {
            match self.load_tile_file(&file, namespace) {
                Ok(_) => {},
                Err(err) => err.log_err(),
            }
        }
    }

    pub fn compile_ordered_tiles(&self) -> Vec<TileId> {
        let mut ids = self.registry.tile_defs.keys().cloned().collect::<Vec<_>>();
        ids.sort_by_key(|id| self.tile_name(*id));

        if let Some(i) = ids
            .iter()
            .enumerate()
            .find_map(|(index, id)| if id.is_none() { Some(index) } else { None })
        {
            let old = ids.remove(i);
            ids.insert(0, old);
        }

        ids
    }
}
