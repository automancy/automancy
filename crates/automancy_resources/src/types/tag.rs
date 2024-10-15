use crate::registry::Registry;
use crate::{load_recursively, ResourceManager, RON_EXT};
use automancy_defs::{id::Id, parse_ids};
use hashbrown::HashSet;
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct TagDef {
    pub id: Id,
    pub entries: HashSet<Id>,
}

impl TagDef {
    pub fn of(&self, registry: &Registry, id: Id) -> bool {
        if self.id == registry.any {
            true
        } else {
            self.entries.contains(&id)
        }
    }
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: String,
    pub entries: Vec<String>,
}

impl ResourceManager {
    fn load_tag(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading tag at: {file:?}");

        let v = ron::from_str::<Raw>(&read_to_string(file)?)?;

        let id = Id::parse(&v.id, &mut self.interner, Some(namespace)).unwrap();

        self.registry.tags.insert(
            id,
            TagDef {
                id,
                entries: parse_ids(v.entries.into_iter(), &mut self.interner, Some(namespace)),
            },
        );

        Ok(())
    }

    pub fn load_tags(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let tags = dir.join("tags");

        for file in load_recursively(&tags, OsStr::new(RON_EXT)) {
            self.load_tag(&file, namespace)?;
        }

        Ok(())
    }
}
