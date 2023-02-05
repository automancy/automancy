use crate::resource::{ResourceManager, JSON_EXT};
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

use serde::Deserialize;

use crate::game::item::{Item, ItemRaw};
use crate::util::id::{Id, IdRaw};

#[derive(Debug, Clone, Copy)]
pub struct Script {
    pub id: Id,
    pub instructions: Instructions,
}

#[derive(Debug, Clone, Copy)]
pub struct Instructions {
    pub input: Option<Item>,
    pub output: Option<Item>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptRaw {
    pub id: IdRaw,
    pub instructions: InstructionsRaw,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstructionsRaw {
    pub input: Option<ItemRaw>,
    pub output: Option<ItemRaw>,
}

impl ResourceManager {
    fn load_script(&mut self, file: &Path) -> Option<()> {
        log::info!("loading script at: {file:?}");

        let script: ScriptRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

        let id = script.id.to_id(&mut self.interner);

        let instructions = Instructions {
            input: script
                .instructions
                .input
                .as_ref()
                .map(|v| v.to_item(&mut self.interner)),
            output: script
                .instructions
                .output
                .as_ref()
                .map(|v| v.to_item(&mut self.interner)),
        };

        let script = Script { id, instructions };

        self.scripts.insert(id, script);

        Some(())
    }

    pub fn load_scripts(&mut self, dir: &Path) -> Option<()> {
        let scripts = dir.join("scripts");
        let scripts = read_dir(scripts).ok()?;

        scripts
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .for_each(|script| {
                self.load_script(&script);
            });

        Some(())
    }
}
