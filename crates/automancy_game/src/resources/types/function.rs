use std::{ffi::OsStr, path::Path};

use automancy_data::id::{Id, deserialize::IdStr};
use hashbrown::HashSet;
use rhai::{ImmutableString, Module, Scope};

use crate::resources::{FUNCTION_EXT, ResourceManager, load_recursively};

pub struct FunctionMetadata {
    pub str_id: String,
}

impl ResourceManager {
    pub fn load_functions(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let functions = dir.join("functions");

        {
            let lib = functions.join("lib");

            for file in load_recursively(&lib, OsStr::new(FUNCTION_EXT)) {
                log::info!("Loading library function at {file:?}");

                let Some(name) = file.file_stem().and_then(OsStr::to_str).map(str::to_string)
                else {
                    continue;
                };

                let name = format!("lib::{}::{}", namespace, name);

                let pre_ast = self.engine.compile_file(file.clone())?;

                let id_deps = self.engine.call_fn::<rhai::Array>(
                    &mut Scope::new(),
                    &pre_ast,
                    "id_deps",
                    (),
                )?;
                let mut scope = Scope::new();
                for id_dep in id_deps.into_iter() {
                    let v = id_dep.cast::<rhai::Array>();

                    let id = IdStr::parse(
                        v[0].clone().cast::<ImmutableString>().as_str(),
                        Some(namespace),
                    )
                    .unwrap();

                    let key = v[1].clone().cast::<ImmutableString>();

                    log::info!("Adding {key} -> {id} into scope of library function {name}");

                    scope.push_constant(
                        key.as_str(),
                        Id::parse(&id, &mut self.interner, Some(namespace)).unwrap(),
                    );
                }

                let module = Module::eval_ast_as_new(
                    Scope::new(),
                    &self.engine.compile_file_with_scope(&scope, file)?,
                    &self.engine,
                );

                match module {
                    Ok(module) => {
                        self.engine
                            .register_static_module(name.clone(), module.into());
                    }
                    Err(err) => {
                        log::error!("Could not register library function {name}! Error: {err:?}");
                        continue;
                    }
                }

                log::info!("Registered library function with name '{name}'!");
            }
        }

        {
            let src = functions.join("src");

            for file in load_recursively(&src, OsStr::new(FUNCTION_EXT)) {
                log::info!("Loading source function at {file:?}");

                let mut scope = Scope::new();
                let pre_ast = self.engine.compile_file(file.clone())?;

                let raw_id = self.engine.call_fn::<ImmutableString>(
                    &mut scope,
                    &pre_ast,
                    "function_id",
                    (),
                )?;
                let raw_id = IdStr::parse(&raw_id, Some(namespace)).unwrap();
                let str_id = raw_id.to_string();

                let id = raw_id.into_id(&mut self.interner);

                let id_deps = self.engine.call_fn::<rhai::Array>(
                    &mut Scope::new(),
                    &pre_ast,
                    "id_deps",
                    (),
                )?;
                let mut scope = Scope::new();
                for id_dep in id_deps.into_iter() {
                    let v = id_dep.cast::<rhai::Array>();

                    let id = IdStr::parse(
                        v[0].clone().cast::<ImmutableString>().as_str(),
                        Some(namespace),
                    )
                    .unwrap();

                    let key = v[1].clone().cast::<ImmutableString>();

                    log::info!("Adding {key} -> {id} into scope of source function {str_id}");

                    scope.push_constant(
                        key.as_str(),
                        Id::parse(&id, &mut self.interner, Some(namespace)).unwrap(),
                    );
                }

                let ast = self.engine.compile_file_with_scope(&scope, file)?;

                let metadata = FunctionMetadata {
                    str_id: str_id.clone(),
                };

                self.functions.insert(id, (ast, metadata));

                log::info!("Registered source function with ID '{str_id}'!");
            }
        }

        Ok(())
    }
}
