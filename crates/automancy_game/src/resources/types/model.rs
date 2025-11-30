use std::{fs::read_to_string, path::Path};

use automancy_data::id::{ItemId, ModelId, deserialize::StrId};
use gltf;
use log;
use serde::Deserialize;

use crate::{
    persistent,
    resources::{MutableResourceManager, RON_EXTS, ResourceError, ResourceManager, read_recursively},
};

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: StrId,
    pub file: String,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_model_file(&mut self, file: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading model at: {}.", file.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = ModelId(self.interner.get_or_intern(v.id, Some(namespace))?);

        let file = file.parent().unwrap().join("files").join(v.file.as_str());
        log::info!("Loading model file at: {}.", file.display());
        let (document, buffers, _images) = gltf::import(file)?;

        self.models.insert(id, (document, buffers));

        Ok(())
    }

    pub fn load_model_files(&mut self, dir: &Path, namespace: &str) {
        let path = dir.join("models");

        for file in read_recursively(&path, RON_EXTS) {
            match self.load_model_file(&file, namespace) {
                Ok(_) => {},
                Err(err) => err.log_err(),
            }
        }
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl ResourceManager {
    pub fn model_or_missing_tile(&self, id: ModelId) -> ModelId {
        if !id.is_none() && self.models.contains_key(&id) {
            id
        } else {
            self.registry.model_ids.tile_missing
        }
    }

    pub fn model_or_missing_item(&self, id: ModelId) -> ModelId {
        if !id.is_none() && self.models.contains_key(&id) {
            id
        } else {
            self.registry.model_ids.item_missing
        }
    }

    pub fn model_or_puzzle_space(&self, id: ModelId) -> ModelId {
        if !id.is_none() && self.models.contains_key(&id) {
            id
        } else {
            self.registry.model_ids.puzzle_space
        }
    }

    pub fn item_model_or_missing(&self, id: ItemId) -> ModelId {
        if let Some(v) = self.registry.item_defs.get(&id) {
            self.model_or_missing_item(v.model)
        } else {
            self.registry.model_ids.item_missing
        }
    }
}
