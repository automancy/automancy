use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_data::id::{Id, ModelId, deserialize::StrId};
use serde::Deserialize;

use crate::{
    persistent,
    resources::{RON_EXT, ResourceManager, load_recursively},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ItemDef {
    pub id: Id,
    pub model: ModelId,
}

#[derive(Debug, Deserialize)]
struct Raw {
    id: StrId,
    model: StrId,
}

impl ResourceManager {
    fn load_item(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading item at: {file:?}");

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = v.id.into_id(&mut self.interner, Some(namespace))?;
        let model = v.model.into_id(&mut self.interner, Some(namespace))?;

        self.registry.item_defs.insert(id, ItemDef { id, model: ModelId(model) });

        Ok(())
    }

    pub fn load_items(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let items = dir.join("items");

        for file in load_recursively(&items, OsStr::new(RON_EXT)) {
            self.load_item(&file, namespace)?;
        }

        Ok(())
    }

    pub fn ordered_items(&mut self) {
        let mut ids = self.registry.item_defs.keys().cloned().collect::<Vec<_>>();

        ids.sort_by_key(|id| self.item_name(*id));

        self.ordered_items = ids;
    }
}
