use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::Deserialize;

use automancy_defs::id::Id;

use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ItemDef {
    pub id: Id,
    pub model: Id,
}

#[derive(Debug, Deserialize)]
struct Raw {
    id: String,
    model: String,
}

impl ResourceManager {
    fn load_item(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading item at: {file:?}");

        let v = ron::from_str::<Raw>(&read_to_string(file)?)?;

        let id = Id::parse(&v.id, &mut self.interner, Some(namespace)).unwrap();
        let model = Id::parse(&v.model, &mut self.interner, Some(namespace)).unwrap();

        self.registry.items.insert(id, ItemDef { id, model });

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
        let mut ids = self.registry.items.keys().cloned().collect::<Vec<_>>();

        ids.sort_by_key(|id| self.item_name(*id));

        self.ordered_items = ids;
    }
}
