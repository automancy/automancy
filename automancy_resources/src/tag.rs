use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::Deserialize;

use automancy_defs::hashbrown::HashSet;
use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::registry::Registry;
use crate::{load_recursively, ResourceManager, JSON_EXT};

#[derive(Debug, Clone, Deserialize)]
pub struct TagJson {
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
    fn load_tag(&mut self, file: &Path) -> anyhow::Result<()> {
        log::info!("loading tag at: {file:?}");

        let tag: TagJson = serde_json::from_str(&read_to_string(file)?)?;

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

        Ok(())
    }

    pub fn load_tags(&mut self, dir: &Path) -> anyhow::Result<()> {
        let tags = dir.join("tags");

        for file in load_recursively(&tags, OsStr::new(JSON_EXT)) {
            self.load_tag(&file)?;
        }

        Ok(())
    }
}
