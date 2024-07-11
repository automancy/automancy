use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;

use automancy_defs::id::Id;

use hashbrown::HashMap;

use crate::{item_match, load_recursively, ResourceManager, RON_EXT};

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

    pub fn get_items(
        &self,
        id: Id,
        tag_cache: &mut HashMap<Id, Arc<Vec<ItemDef>>>,
    ) -> Arc<Vec<ItemDef>> {
        if let Some(item) = self.registry.items.get(&id) {
            Arc::new(vec![*item])
        } else {
            tag_cache
                .entry(id)
                .or_insert_with(|| {
                    let items = self
                        .ordered_items
                        .iter()
                        .filter(|v| item_match(self, **v, id))
                        .flat_map(|v| self.registry.items.get(v).cloned())
                        .collect();

                    Arc::new(items)
                })
                .clone()
        }
    }

    pub fn ordered_items(&mut self) {
        let mut ids = self.registry.items.keys().cloned().collect::<Vec<_>>();

        ids.sort_by_key(|id| self.item_name(*id));

        self.ordered_items = ids;
    }
}
