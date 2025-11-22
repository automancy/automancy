use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_data::{
    game::inventory::{ItemStack, deserialize::ItemStackStr},
    id::{Id, deserialize::StrId, parse::parse_item_stacks},
};
use serde::Deserialize;

use crate::{
    persistent,
    resources::{RON_EXT, ResourceManager, load_recursively},
};

#[derive(Debug, Clone)]
pub struct RecipeDef {
    pub id: Id,
    pub inputs: Option<Vec<ItemStack>>,
    pub outputs: Vec<ItemStack>,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: StrId,
    pub inputs: Option<Vec<ItemStackStr>>,
    pub output: Vec<ItemStackStr>,
}

impl ResourceManager {
    fn load_recipe(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading recipe at: {file:?}");

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = v.id.into_id(&mut self.interner, Some(namespace))?;

        let inputs = match v.inputs {
            Some(v) => Some(parse_item_stacks(v.into_iter(), &mut self.interner, Some(namespace)).try_collect()?),
            None => None,
        };
        let outputs = parse_item_stacks(v.output.into_iter(), &mut self.interner, Some(namespace)).try_collect()?;

        let script = RecipeDef { id, inputs, outputs };

        self.registry.recipe_defs.insert(id, script);

        Ok(())
    }

    pub fn load_recipes(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let scripts = dir.join("recipes");

        for file in load_recursively(&scripts, OsStr::new(RON_EXT)) {
            self.load_recipe(&file, namespace)?;
        }

        Ok(())
    }
}
