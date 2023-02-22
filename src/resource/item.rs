use crate::resource::{Registry, ResourceManager, JSON_EXT};
use crate::util::id::{Id, IdRaw, Interner};
use rune::Any;
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

impl ItemRaw {
    pub fn to_item(self, interner: &mut Interner) -> Item {
        Item {
            id: self.id.to_id(interner),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Any)]
pub struct Item {
    #[rune(get, copy)]
    pub id: Id,
}

pub fn id_eq_or_of_tag(registry: &Registry, id: Id, other: Id) -> bool {
    if id == other {
        return true;
    }

    if let Some(tag) = registry.tags.get(&other) {
        return tag.of(registry, id);
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

        let item = item.to_item(&mut self.interner);

        self.registry.items.insert(item.id, item);

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
        if let Some(item) = self.registry.items.get(&id) {
            Arc::new(vec![*item])
        } else {
            tag_cache.entry(id).or_insert_with(|| {
                let items = self
                    .ordered_items
                    .iter()
                    .filter(|v| id_eq_or_of_tag(&self.registry, **v, id))
                    .map(|v| Item { id: *v })
                    .collect();

                Arc::new(items)
            });

            tag_cache[&id].clone()
        }
    }

    pub fn ordered_items(&mut self) {
        let mut ids = self.registry.items.keys().cloned().collect::<Vec<_>>();

        ids.sort_unstable_by_key(|id| self.item_name(id));

        self.ordered_items = ids;
    }
}
