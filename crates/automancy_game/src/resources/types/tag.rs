use std::{fs::read_to_string, path::Path};

use automancy_data::{
    id::{Id, IdInterner, TagId, deserialize::StrId, parse::parse_ids},
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
    fn load_tag_file(&mut self, interner: &mut IdInterner, path: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading tag at: {}.", path.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(path)?)?;

        let id = TagId(interner.get_or_intern(&v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "tag",
                file: path.to_path_buf(),
                name: interner.resolve(*id).unwrap().to_string(),
            });
        }
        let entries = parse_ids(v.entries.into_iter(), interner, Some(namespace)).try_collect()?;

        self.registry.tag_defs.insert(
            id,
            TagDef {
                id,
                entries,
            },
        );

        Ok(())
    }

    pub fn load_tag_files(dir: &Path, namespace: &str) {
        MutableResourceManager::with_interner(|resource_man, interner| {
            let path = dir.join("tags");

            for entry in read_recursively(&path, RON_EXTS) {
                match resource_man.load_tag_file(interner, entry.path(), namespace) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
    }
}
