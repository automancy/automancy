use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

use crate::resource::ResourceManager;
use crate::resource::{Deserialize, JSON_EXT};
use crate::util::id::{Id, IdRaw};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TranslateRaw {
    pub items: HashMap<IdRaw, String>,
    pub tiles: HashMap<IdRaw, String>,
    pub gui: HashMap<IdRaw, String>,
}

#[derive(Debug, Default, Clone)]
pub struct Translate {
    pub items: HashMap<Id, String>,
    pub tiles: HashMap<Id, String>,
    pub gui: HashMap<Id, String>,
}
impl ResourceManager {
    fn load_translate(&mut self, file: &Path) -> Option<()> {
        log::info!("loading translate at: {file:?}");

        let translate: TranslateRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

        let items = translate
            .items
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut self.interner), str))
            .collect();
        let tiles = translate
            .tiles
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut self.interner), str))
            .collect();
        let gui = translate
            .gui
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut self.interner), str))
            .collect();
        self.translates = Translate { items, tiles, gui };

        Some(())
    }
    pub fn load_translates(&mut self, dir: &Path) -> Option<()> {
        let translates = dir.join("translates");
        let translates = read_dir(translates).ok()?;

        translates
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .for_each(|translate| {
                // TODO language selection
                if translate.file_stem() == Some(OsStr::new("en_US")) {
                    self.load_translate(&translate);
                }
            });

        Some(())
    }
}
