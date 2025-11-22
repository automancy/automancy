use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_data::id::{Id, deserialize::StrId, parse::parse_ids};
use hashbrown::HashSet;
use serde::Deserialize;

use crate::{
    persistent,
    resources::{RON_EXT, ResourceManager, load_recursively, registry::Registry},
};

#[derive(Debug, Clone)]
pub struct TagDef {
    pub id: Id,
    pub entries: HashSet<Id>,
}

impl TagDef {
    pub fn of(&self, registry: &Registry, id: Id) -> bool {
        if self.id == registry.any { true } else { self.entries.contains(&id) }
    }
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: StrId,
    pub entries: Vec<StrId>,
}

impl ResourceManager {
    fn load_tag(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading tag at: {file:?}");

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = v.id.into_id(&mut self.interner, Some(namespace))?;

        self.registry.tag_defs.insert(
            id,
            TagDef {
                id,
                entries: { parse_ids(v.entries.into_iter(), &mut self.interner, Some(namespace)).try_collect()? },
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
