use std::{fs::read_to_string, path::Path};

use automancy_data::id::{ItemId, ModelId, deserialize::StrId};
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
    fn load_item_file(&mut self, file: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading item at: {}.", file.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = ItemId(self.interner.get_or_intern(v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "item",
                file: file.to_path_buf(),
                name: self.interner.resolve(*id).unwrap().to_string(),
            });
        }
        let model = self.interner.get_or_intern(v.model, Some(namespace))?;

        self.registry.item_defs.insert(
            id,
            ItemDef {
                id,
                model: ModelId(model),
            },
        );

        Ok(())
    }

    pub fn load_item_files(&mut self, dir: &Path, namespace: &str) {
        let path = dir.join("items");

        for file in read_recursively(&path, RON_EXTS) {
            match self.load_item_file(&file, namespace) {
                Ok(_) => {},
                Err(err) => err.log_err(),
            }
        }
    }

    pub fn compile_ordered_items(&self) -> Vec<ItemId> {
        let mut ids = self.registry.item_defs.keys().cloned().collect::<Vec<_>>();
        ids.sort_by_key(|id| self.item_name(*id));
        ids
    }
}
