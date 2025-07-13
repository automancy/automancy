use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_data::id::{Id, ModelId, deserialize::StrId};
use gltf;
use log;
use serde::Deserialize;

use crate::{
    persistent,
    resources::{RON_EXT, ResourceManager, load_recursively},
};

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: StrId,
    pub file: String,
}

impl ResourceManager {
    pub fn model_or_missing_tile(&self, id: &ModelId) -> ModelId {
        if self.gltf_models.contains_key(id) {
            *id
        } else {
            ModelId(self.registry.model_ids.tile_missing)
        }
    }

    pub fn model_or_missing_item(&self, id: &ModelId) -> ModelId {
        if self.gltf_models.contains_key(id) {
            *id
        } else {
            ModelId(self.registry.model_ids.item_missing)
        }
    }

    pub fn model_or_puzzle_space(&self, id: &ModelId) -> ModelId {
        if self.gltf_models.contains_key(id) {
            *id
        } else {
            ModelId(self.registry.model_ids.puzzle_space)
        }
    }

    pub fn item_model_or_missing(&self, id: &Id) -> ModelId {
        if let Some(def) = self.registry.item_defs.get(id)
            && self.gltf_models.contains_key(&def.model)
        {
            return def.model;
        }

        ModelId(self.registry.model_ids.item_missing)
    }

    fn load_model(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading model at: {file:?}");

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let file = file.parent().unwrap().join("files").join(v.file.as_str());

        log::info!("Loading model file at: {file:?}");

        let (document, buffers, _images) = gltf::import(file)?;

        let id = v.id.into_id(&mut self.interner, Some(namespace))?;

        self.gltf_models.insert(ModelId(id), (document, buffers));

        Ok(())
    }

    pub fn load_models(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let models = dir.join("models");

        for file in load_recursively(&models, OsStr::new(RON_EXT)) {
            self.load_model(&file, namespace)?;
        }

        Ok(())
    }
}
