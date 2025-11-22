use std::{ffi::OsStr, path::Path};

use automancy_data::id::deserialize::StrId;

use crate::resources::{ResourceManager, SCRIPT_EXT, load_recursively};

pub enum ScriptType {
    Library,
    Tile,
}

pub struct ScriptMetadata {
    pub str_id: String,
}

pub struct ScriptData {
    pub ast: rhai::AST,
    pub metadata: ScriptMetadata,
}

impl ResourceManager {
    fn read_id_deps(
        &mut self,
        namespace: &str,
        str_id: &str,
        script_type: ScriptType,
        ast: &rhai::AST,
        scope: &mut rhai::Scope,
    ) -> anyhow::Result<()> {
        let id_deps = self.engine.call_fn::<rhai::Array>(&mut rhai::Scope::new(), ast, "id_deps", ())?;

        for id_dep in id_deps.into_iter() {
            let mut array = id_dep.cast::<rhai::Array>().into_iter();

            let id = StrId::from(array.next().unwrap().cast::<rhai::ImmutableString>().to_string()).into_id(&mut self.interner, Some(namespace))?;

            let key = array.next().unwrap().cast::<rhai::ImmutableString>();

            match script_type {
                ScriptType::Library => {
                    log::info!("Adding {key} -> {id} into scope of library script {str_id}");
                }
                ScriptType::Tile => {
                    log::info!("Adding {key} -> {id} into scope of tile script {str_id}");
                }
            };

            scope.push_constant(key.as_str(), id);
        }

        Ok(())
    }

    pub fn load_scripts(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let scripts = dir.join("scripts");

        {
            let lib = scripts.join("lib");

            for file in load_recursively(&lib, OsStr::new(SCRIPT_EXT)) {
                log::info!("Loading library script at {file:?}");

                let Some(name) = file.file_stem().and_then(OsStr::to_str).map(str::to_string) else {
                    continue;
                };

                let str_id = format!("lib::{}::{}", namespace, name);
                let mut scope = rhai::Scope::new();

                {
                    let ast = self.engine.compile_file(file.clone())?;

                    self.read_id_deps(namespace, &str_id, ScriptType::Library, &ast, &mut scope)?;
                }

                let ast = self.engine.compile_file_with_scope(&scope, file)?;
                let module = rhai::Module::eval_ast_as_new(rhai::Scope::new(), &ast, &self.engine);

                match module {
                    Ok(module) => {
                        self.engine.register_static_module(str_id.clone(), module.into());
                    }
                    Err(err) => {
                        log::error!("Could not register library script {str_id}! Error: {err:?}");
                        continue;
                    }
                }

                log::info!("Registered library script with id '{str_id}'!");
            }
        }

        let src = scripts.join("src");

        {
            let src_tile = src.join("tile");

            for file in load_recursively(&src_tile, OsStr::new(SCRIPT_EXT)) {
                log::info!("Loading tile script at {file:?}");

                let mut scope = rhai::Scope::new();
                let str_id;
                let id;

                {
                    let ast = self.engine.compile_file(file.clone())?;

                    let raw_id = self
                        .engine
                        .call_fn::<rhai::ImmutableString>(&mut rhai::Scope::new(), &ast, "script_id", ())?;
                    str_id = raw_id.to_string();
                    id = StrId::from(raw_id.to_string()).into_id(&mut self.interner, Some(namespace))?;

                    self.read_id_deps(namespace, &str_id, ScriptType::Tile, &ast, &mut scope)?;
                }

                self.scripts.insert(
                    id,
                    ScriptData {
                        ast: self.engine.compile_file_with_scope(&scope, file)?,
                        metadata: ScriptMetadata { str_id: str_id.clone() },
                    },
                );

                log::info!("Registered tile script with ID '{str_id}'!");
            }
        }

        Ok(())
    }
}
