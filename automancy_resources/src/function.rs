use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::Path;

use rhai::{Dynamic, ImmutableString, Scope};

use automancy_defs::id::IdRaw;
use automancy_defs::log;

use crate::{ResourceManager, FUNCTION_EXT};

impl ResourceManager {
    pub fn load_functions(&mut self, dir: &Path) -> anyhow::Result<()> {
        let functions = dir.join("functions");
        if let Ok(functions) = read_dir(functions) {
            for file in functions
                .into_iter()
                .flatten()
                .map(|v| v.path())
                .filter(|v| v.extension() == Some(OsStr::new(FUNCTION_EXT)))
            {
                log::info!("Loading function at {file:?}");
                let mut scope = Scope::new();
                let ast = self.engine.compile_file(file)?;

                let str_id =
                    self.engine
                        .call_fn::<ImmutableString>(&mut scope, &ast, "function_id", ())?;
                let str_id = IdRaw::parse(&str_id).to_string();
                let id = self.interner.get_or_intern(&str_id);

                let id_deps = self
                    .engine
                    .call_fn::<Dynamic>(&mut scope, &ast, "id_deps", ())?;

                if let Some(id_deps) = id_deps.try_cast::<rhai::Array>() {
                    id_deps.into_iter().for_each(|v| {
                        let v = v.cast::<rhai::Array>();

                        let id = IdRaw::parse(v[0].clone().cast::<ImmutableString>().as_str())
                            .to_string();
                        let key = v[1].clone().cast::<ImmutableString>();

                        log::info!("Adding {key} -> {id} into scope of function {str_id}");

                        scope.push_constant(key.as_str(), self.interner.get_or_intern(&id));
                    });
                }

                self.functions.insert(id, (ast, scope));

                log::info!("Registered function with id {str_id} ({id:?})");
            }
        }

        Ok(())
    }
}
