use crate::resource::{Deserialize, JSON_EXT};
use crate::resource::{LoadResource, ResourceManager};
use crate::util::id::{Id, IdRaw};
use std::any::Any;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TranslateRaw {
    pub items: HashMap<IdRaw, String>,
    pub tiles: HashMap<IdRaw, String>,
}

#[derive(Debug, Default, Clone)]
pub struct Translate {
    pub items: HashMap<Id, String>,
    pub tiles: HashMap<Id, String>,
}
impl LoadResource<Translate> for ResourceManager {
    fn load(resource_man: &mut ResourceManager, file: &Path) -> Option<()> {
        log::info!("loading translate at: {:?}", file);

        let translate: TranslateRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

        let items = translate
            .items
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut resource_man.interner), str))
            .collect();
        let tiles = translate
            .tiles
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut resource_man.interner), str))
            .collect();

        resource_man.translates = Translate { items, tiles };

        Some(())
    }
    const FILTER: dyn FnMut(&PathBuf) -> bool = (|v| v.extension() == Some(OsStr::new(JSON_EXT)));
    const DIR: String = String::from("translates");
}
