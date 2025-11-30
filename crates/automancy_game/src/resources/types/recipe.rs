use std::{fs::read_to_string, path::Path};

use automancy_data::{
    game::inventory::{ItemStack, deserialize::ItemStackStr},
    id::{RecipeId, deserialize::StrId, parse::parse_item_stacks},
};
use serde::Deserialize;

use crate::{
    persistent,
    resources::{MutableResourceManager, RON_EXTS, ResourceError, read_recursively},
};

#[derive(Debug, Clone)]
pub struct RecipeDef {
    pub id: RecipeId,
    pub inputs: Option<Vec<ItemStack>>,
    pub outputs: Vec<ItemStack>,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: StrId,
    pub inputs: Option<Vec<ItemStackStr>>,
    pub output: Vec<ItemStackStr>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_recipe_file(&mut self, file: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading recipe at: {}.", file.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = RecipeId(self.interner.get_or_intern(v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "recipe",
                file: file.to_path_buf(),
                name: self.interner.resolve(*id).unwrap().to_string(),
            });
        }
        let inputs = match v.inputs {
            Some(v) => Some(parse_item_stacks(v.into_iter(), &mut self.interner, Some(namespace)).try_collect()?),
            None => None,
        };
        let outputs = parse_item_stacks(v.output.into_iter(), &mut self.interner, Some(namespace)).try_collect()?;

        self.registry.recipe_defs.insert(
            id,
            RecipeDef {
                id,
                inputs,
                outputs,
            },
        );

        Ok(())
    }

    pub fn load_recipe_files(&mut self, dir: &Path, namespace: &str) {
        let path = dir.join("recipes");

        for file in read_recursively(&path, RON_EXTS) {
            match self.load_recipe_file(&file, namespace) {
                Ok(_) => {},
                Err(err) => err.log_err(),
            }
        }
    }
}
