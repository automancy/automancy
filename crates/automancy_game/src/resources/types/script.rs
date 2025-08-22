use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_data::{
    game::item::{ItemAmount, ItemStack},
    id::{Id, parse::parse_item_stacks},
};
use serde::Deserialize;

use crate::resources::{RON_EXT, ResourceManager, load_recursively};

#[derive(Debug, Clone)]
pub struct InstructionsDef {
    pub inputs: Option<Vec<ItemStack>>,
    pub outputs: Vec<ItemStack>,
}

#[derive(Debug, Clone)]
pub struct ScriptDef {
    pub id: Id,
    pub instructions: InstructionsDef,
}

#[derive(Debug, Deserialize)]
struct InstructionsRaw {
    pub inputs: Option<Vec<(String, ItemAmount)>>,
    pub output: Vec<(String, ItemAmount)>,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: String,
    pub instructions: InstructionsRaw,
}

impl ResourceManager {
    fn load_script(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading script at: {file:?}");

        let v = ron::from_str::<Raw>(&read_to_string(file)?)?;

        let id = Id::parse(&v.id, &mut self.interner, Some(namespace)).unwrap();

        let instructions = InstructionsDef {
            inputs: v
                .instructions
                .inputs
                .map(|v| parse_item_stacks(v.into_iter(), &mut self.interner, Some(namespace))),
            outputs: parse_item_stacks(
                v.instructions.output.into_iter(),
                &mut self.interner,
                Some(namespace),
            ),
        };

        let script = ScriptDef { id, instructions };

        self.registry.scripts.insert(id, script);

        Ok(())
    }

    pub fn load_scripts(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let scripts = dir.join("scripts");

        for file in load_recursively(&scripts, OsStr::new(RON_EXT)) {
            self.load_script(&file, namespace)?;
        }

        Ok(())
    }
}
