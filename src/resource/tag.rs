use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use egui::epaint::ahash::HashSet;
use serde::{Deserialize, Serialize};

use crate::resource::{load_recursively, Registry, ResourceManager, JSON_EXT};
use crate::util::id::{Id, IdRaw};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TagRaw {
    pub id: IdRaw,
    pub entries: Vec<IdRaw>,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: Id,
    pub entries: HashSet<Id>,
}

impl Tag {
    pub fn of(&self, registry: &Registry, id: Id) -> bool {
        if self.id == registry.any {
            true
        } else {
            self.entries.contains(&id)
        }
    }
}

impl ResourceManager {
    fn load_tag(&mut self, file: &Path) -> Option<()> {
        log::info!("loading tag at: {file:?}");

        let tag: TagRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
        )
        .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}"));

        let id = tag.id.to_id(&mut self.interner);

        let tag = Tag {
            id,
            entries: tag
                .entries
                .into_iter()
                .map(|id| id.to_id(&mut self.interner))
                .collect(),
        };

        self.registry.tags.insert(id, tag);

        Some(())
    }

    pub fn load_tags(&mut self, dir: &Path) -> Option<()> {
        let tags = dir.join("tags");

        load_recursively(&tags, OsStr::new(JSON_EXT))
            .into_iter()
            .for_each(|file| {
                self.load_tag(&file);
            });

        Some(())
    }
}
