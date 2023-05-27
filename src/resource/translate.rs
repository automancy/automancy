use hashbrown::HashMap;
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

use flexstr::{SharedStr, ToSharedStr};

use crate::resource::ResourceManager;
use crate::resource::{Deserialize, JSON_EXT};
use crate::util::id::{Id, IdRaw};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TranslateRaw {
    none: String,
    unnamed: String,
    items: HashMap<IdRaw, String>,
    tiles: HashMap<IdRaw, String>,
    scripts: HashMap<IdRaw, String>,
    gui: HashMap<IdRaw, String>,
    error: HashMap<IdRaw, String>,
}

#[derive(Debug, Default, Clone)]
pub struct Translate {
    pub none: SharedStr,
    pub unnamed: SharedStr,
    pub items: HashMap<Id, SharedStr>,
    pub tiles: HashMap<Id, SharedStr>,
    pub scripts: HashMap<Id, SharedStr>,
    pub gui: HashMap<Id, SharedStr>,
    pub error: HashMap<Id, SharedStr>,
}

impl ResourceManager {
    fn load_translate(&mut self, file: &Path) -> Option<()> {
        log::info!("loading translate at: {file:?}");

        let translate: TranslateRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
        )
        .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}"));

        let none = translate.none.to_shared_str();
        let unnamed = translate.unnamed.to_shared_str();

        let items = translate
            .items
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut self.interner), str.into()))
            .collect();
        let tiles = translate
            .tiles
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut self.interner), str.into()))
            .collect();
        let scripts = translate
            .scripts
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut self.interner), str.into()))
            .collect();
        let gui = translate
            .gui
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut self.interner), str.into()))
            .collect();
        let error = translate
            .error
            .into_iter()
            .map(|(id, str)| (id.to_id(&mut self.interner), str.into()))
            .collect();
        self.translates = Translate {
            none,
            unnamed,
            items,
            tiles,
            scripts,
            gui,
            error,
        };

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
