use std::{fs::read_to_string, path::Path};

use automancy_data::id::{IdInterner, ItemId, ModelId, deserialize::StrId};
use serde::Deserialize;

use crate::{
    persistent,
    resources::{MutableResourceManager, RON_EXTS, ResourceError, read_recursively},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ItemDef {
    pub id: ItemId,
    pub model: ModelId,
}

#[derive(Debug, Deserialize)]
struct Raw {
    id: StrId,
    model: StrId,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_item_file(&mut self, interner: &mut IdInterner, path: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading item at: {}.", path.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(path)?)?;

        let id = ItemId(interner.get_or_intern(&v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "item",
                file: path.to_path_buf(),
                name: interner.resolve(*id).unwrap().to_string(),
            });
        }
        let model = interner.get_or_intern(&v.model, Some(namespace))?;

        self.registry.item_defs.insert(
            id,
            ItemDef {
                id,
                model: ModelId(model),
            },
        );

        Ok(())
    }

    pub fn load_item_files(dir: &Path, namespace: &str) {
        MutableResourceManager::with_interner(|resource_man, interner| {
            let path = dir.join("items");

            for entry in read_recursively(&path, RON_EXTS) {
                match resource_man.load_item_file(interner, entry.path(), namespace) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
    }

    pub fn compile_ordered_items(&self) -> Vec<ItemId> {
        let mut ids = self.registry.item_defs.keys().cloned().collect::<Vec<_>>();
        ids.sort_by_key(|id| self.item_name(*id));
        ids
    }
}
