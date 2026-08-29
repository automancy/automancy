use std::{fs::read_to_string, path::Path};

use automancy_data::{
    game::generic::{DataMap, deserialize::DataMapStr},
    id::{CategoryId, IdInterner, ScriptId, TileId, deserialize::StrId},
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
    fn load_tile_file(&mut self, interner: &mut IdInterner, path: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading tile definition at {}.", path.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(path)?)?;

        let id = TileId(interner.get_or_intern(&v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "tile",
                file: path.to_path_buf(),
                name: interner.resolve(*id).unwrap().to_string(),
            });
        }

        let script = {
            match v.script {
                Some(v) => {
                    let (namespace, name) = IdInterner::parse_string_id(&v, Some(namespace))?;
                    let id = format!("{namespace}:tile/{name}");

                    ScriptId(interner.get_or_intern(&id, Some(namespace))?)
                },
                None => ScriptId::none(),
            }
        };
        let category = CategoryId(interner.get_or_intern_opt(v.category.as_deref(), Some(namespace))?);
        let data = v.data.into_data(interner, Some(namespace))?;

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

    pub fn load_tile_files(dir: &Path, namespace: &str) {
        MutableResourceManager::with_interner(|resource_man, interner| {
            let path = dir.join("tiles");

            for entry in read_recursively(&path, RON_EXTS) {
                match resource_man.load_tile_file(interner, entry.path(), namespace) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
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
