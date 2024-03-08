use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;
use hashbrown::HashMap;

use crate::data::item::{item_match, Item};
use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ItemRaw {
    id: IdRaw,
    model: IdRaw,
}

impl ResourceManager {
    fn load_item(&mut self, file: &Path) -> anyhow::Result<()> {
        log::info!("Loading item at: {file:?}");

        let item: ItemRaw = ron::from_str(&read_to_string(file)?)?;

        let id = item.id.to_id(&mut self.interner);
        let model = item.model.to_id(&mut self.interner);

        self.registry.items.insert(id, Item { id, model });

        Ok(())
    }

    pub fn load_items(&mut self, dir: &Path) -> anyhow::Result<()> {
        let items = dir.join("items");

        for file in load_recursively(&items, OsStr::new(RON_EXT)) {
            self.load_item(&file)?;
        }

        Ok(())
    }

    pub fn get_items(&self, id: Id, tag_cache: &mut HashMap<Id, Arc<Vec<Item>>>) -> Arc<Vec<Item>> {
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

        ids.sort_by_key(|id| self.item_name(id));

        self.ordered_items = ids;
    }
}
