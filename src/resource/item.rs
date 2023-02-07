use crate::resource::{ResourceManager, JSON_EXT};
use crate::util::id::{Id, IdRaw};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ItemRaw {
    pub id: IdRaw,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Item {
    pub id: Id,
}

pub fn id_eq_or_of_tag(resource_man: &ResourceManager, item: Id, other: Id) -> bool {
    if item == other {
        return true;
    }

    if let Some(tag) = resource_man.tags.get(&other) {
        return tag.of(resource_man, item);
    }

    false
}

impl ResourceManager {
    fn load_item(&mut self, file: &Path) -> Option<()> {
        log::info!("loading item at: {file:?}");

        let item: ItemRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
        )
        .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}"));

        let id = item.id.to_id(&mut self.interner);

        let item = Item { id };

        self.items.insert(id, item);

        Some(())
    }

    pub fn load_items(&mut self, dir: &Path) -> Option<()> {
        let items = dir.join("items");
        let items = read_dir(items).ok()?;

        items
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .for_each(|item| {
                self.load_item(&item);
            });

        Some(())
    }

    pub fn get_items(&self, id: Id, tag_cache: &mut HashMap<Id, Arc<Vec<Item>>>) -> Arc<Vec<Item>> {
        if let Some(item) = self.items.get(&id) {
            Arc::new(vec![*item])
        } else {
            if !tag_cache.contains_key(&id) {
                let items = self
                    .items
                    .values()
                    .filter(|v| id_eq_or_of_tag(&self, v.id, id))
                    .cloned()
                    .collect();

                tag_cache.insert(id, Arc::new(items));
            }

            tag_cache[&id].clone()
        }
    }
}
