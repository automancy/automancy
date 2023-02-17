use crate::resource::{ResourceManager, JSON_EXT};
use rune::Any;
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

use serde::Deserialize;

use crate::game::item::{ItemStack, ItemStackRaw};
use crate::util::id::{Id, IdRaw};

#[derive(Debug, Clone, Any)]
pub struct Script {
    #[rune(get, copy)]
    pub id: Id,
    #[rune(get)]
    pub instructions: Instructions,
}

#[derive(Debug, Clone, Any)]
pub struct Instructions {
    #[rune(get)]
    pub inputs: Option<Vec<ItemStack>>,
    #[rune(get, copy)]
    pub output: Option<ItemStack>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptRaw {
    pub id: IdRaw,
    pub instructions: InstructionsRaw,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstructionsRaw {
    pub inputs: Option<Vec<ItemStackRaw>>,
    pub output: Option<ItemStackRaw>,
}

impl ResourceManager {
    fn load_script(&mut self, file: &Path) -> Option<()> {
        log::info!("loading script at: {file:?}");

        let script: ScriptRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
        )
        .unwrap_or_else(|e| panic!("error loading {file:?}: {e:?}"));

        let id = script.id.to_id(&mut self.interner);

        let instructions = Instructions {
            inputs: script.instructions.inputs.map(|v| {
                v.into_iter()
                    .map(|item_stack| item_stack.to_item(&mut self.interner))
                    .collect()
            }),
            output: script
                .instructions
                .output
                .map(|v| v.to_item(&mut self.interner)),
        };

        let script = Script { id, instructions };

        self.registry.scripts.insert(id, script);

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
