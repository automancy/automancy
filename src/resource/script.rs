use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::Deserialize;

use crate::game::item::{ItemAmount, ItemStack};
use crate::resource::{load_recursively, ResourceManager, JSON_EXT};
use crate::util::id::{Id, IdRaw};

#[derive(Debug, Clone)]
pub struct Script {
    pub id: Id,

    pub adjacent: Option<Id>,
    pub instructions: Instructions,
}

#[derive(Debug, Clone)]
pub struct Instructions {
    pub inputs: Option<Vec<ItemStack>>,
    pub output: ItemStack,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptRaw {
    pub id: IdRaw,
    pub adjacent: Option<IdRaw>,
    pub instructions: InstructionsRaw,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstructionsRaw {
    pub inputs: Option<Vec<(IdRaw, ItemAmount)>>,
    pub output: (IdRaw, ItemAmount),
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
                    .flat_map(|(id, amount)| {
                        self.registry
                            .item(id.to_id(&mut self.interner))
                            .cloned()
                            .map(|item| ItemStack { item, amount })
                    })
                    .collect()
            }),
            output: ItemStack {
                item: *self
                    .registry
                    .item(script.instructions.output.0.to_id(&mut self.interner))
                    .unwrap(),
                amount: script.instructions.output.1,
            },
        };

        let adjacent = script.adjacent.map(|id| id.to_id(&mut self.interner));

        let script = Script {
            id,
            instructions,
            adjacent,
        };

        self.registry.scripts.insert(id, script);

        Some(())
    }

    pub fn load_scripts(&mut self, dir: &Path) -> Option<()> {
        let scripts = dir.join("scripts");

        load_recursively(&scripts, OsStr::new(JSON_EXT))
            .into_iter()
            .for_each(|file| {
                self.load_script(&file);
            });

        Some(())
    }
}
