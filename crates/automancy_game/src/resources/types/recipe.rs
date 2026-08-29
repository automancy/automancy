use std::{fs::read_to_string, path::Path};

use automancy_data::{
    game::inventory::{ItemStack, deserialize::ItemStackStr},
    id::{IdInterner, RecipeId, deserialize::StrId, parse::parse_item_stacks},
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
    fn load_recipe_file(&mut self, interner: &mut IdInterner, path: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading recipe at: {}.", path.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(path)?)?;

        let id = RecipeId(interner.get_or_intern(&v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "recipe",
                file: path.to_path_buf(),
                name: interner.resolve(*id).unwrap().to_string(),
            });
        }
        let inputs = match v.inputs {
            Some(v) => Some(parse_item_stacks(v.into_iter(), interner, Some(namespace)).try_collect()?),
            None => None,
        };
        let outputs = parse_item_stacks(v.output.into_iter(), interner, Some(namespace)).try_collect()?;

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

    pub fn load_recipe_files(dir: &Path, namespace: &str) {
        MutableResourceManager::with_interner(|resource_man, interner| {
            let path = dir.join("recipes");

            for entry in read_recursively(&path, RON_EXTS) {
                match resource_man.load_recipe_file(interner, entry.path(), namespace) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
    }
}
