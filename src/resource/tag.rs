use crate::resource::{Registry, ResourceManager, JSON_EXT};
use crate::util::id::{Id, IdRaw};
use egui::epaint::ahash::HashSet;
use rune::Any;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TagRaw {
    pub id: IdRaw,
    pub entries: Vec<IdRaw>,
}

#[derive(Debug, Clone, Any)]
pub struct Tag {
    #[rune(get, copy)]
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
        let tags = read_dir(tags).ok()?;

        tags.into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .for_each(|tag| {
                self.load_tag(&tag);
            });

        Some(())
    }
}
