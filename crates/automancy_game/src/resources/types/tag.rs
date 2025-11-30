use std::{fs::read_to_string, path::Path};

use automancy_data::{
    id::{Id, TagId, deserialize::StrId, parse::parse_ids},
    id_map::ImmutableIdSet,
};
use serde::Deserialize;

use crate::{
    persistent,
    resources::{MutableResourceManager, RON_EXTS, ResourceError, read_recursively},
};

#[derive(Debug, Clone)]
pub struct TagDef {
    pub id: TagId,
    pub entries: ImmutableIdSet<Id>,
}

impl TagDef {
    pub fn contains(&self, id: Id) -> bool {
        if self.id.is_any() { true } else { self.entries.contains(&id) }
    }
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: StrId,
    pub entries: Vec<StrId>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_tag_file(&mut self, file: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading tag at: {}.", file.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = TagId(self.interner.get_or_intern(v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "tag",
                file: file.to_path_buf(),
                name: self.interner.resolve(*id).unwrap().to_string(),
            });
        }
        let entries = parse_ids(v.entries.into_iter(), &mut self.interner, Some(namespace)).try_collect()?;

        self.registry.tag_defs.insert(
            id,
            TagDef {
                id,
                entries,
            },
        );

        Ok(())
    }

    pub fn load_tag_files(&mut self, dir: &Path, namespace: &str) {
        let path = dir.join("tags");

        for file in read_recursively(&path, RON_EXTS) {
            match self.load_tag_file(&file, namespace) {
                Ok(_) => {},
                Err(err) => err.log_err(),
            }
        }
    }
}
