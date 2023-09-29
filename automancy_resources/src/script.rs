use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::Deserialize;

use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::data::stack::{ItemAmount, ItemStack};
use crate::{load_recursively, ResourceManager, JSON_EXT};

#[derive(Debug, Clone)]
pub struct Script {
    pub id: Id,

    pub adjacent: Option<Id>,
    pub instructions: Instructions,
}

#[derive(Debug, Clone)]
pub struct Instructions {
    pub inputs: Option<Vec<ItemStack>>,
    pub outputs: Vec<ItemStack>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptJson {
    pub id: IdRaw,
    pub adjacent: Option<IdRaw>,
    pub instructions: InstructionsJson,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstructionsJson {
    pub inputs: Option<Vec<(IdRaw, ItemAmount)>>,
    pub output: Vec<(IdRaw, ItemAmount)>,
}

impl ResourceManager {
    fn load_script(&mut self, file: &Path) -> anyhow::Result<()> {
        log::info!("Loading script at: {file:?}");

        let script: ScriptJson = serde_json::from_str(&read_to_string(file)?)?;

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
            outputs: script
                .instructions
                .output
                .into_iter()
                .flat_map(|(id, amount)| {
                    self.registry
                        .item(id.to_id(&mut self.interner))
                        .cloned()
                        .map(|item| ItemStack { item, amount })
                })
                .collect(),
        };

        let adjacent = script.adjacent.map(|id| id.to_id(&mut self.interner));

        let script = Script {
            id,
            instructions,
            adjacent,
        };

        self.registry.scripts.insert(id, script);

        Ok(())
    }

    pub fn load_scripts(&mut self, dir: &Path) -> anyhow::Result<()> {
        let scripts = dir.join("scripts");

        for file in load_recursively(&scripts, OsStr::new(JSON_EXT)) {
            self.load_script(&file)?;
        }

        Ok(())
    }
}
