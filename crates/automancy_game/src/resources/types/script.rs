use std::path::Path;

use automancy_data::id::ScriptId;

use crate::resources::{MutableResourceManager, ResourceError, SCRIPT_EXTS, read_recursively};

#[derive(Debug)]
pub enum ScriptType {
    Library,
    Tile,
}

#[derive(Debug)]
pub struct ScriptMetadata {
    pub str_id: String,
}

#[derive(Debug)]
pub struct ScriptData {
    pub ast: rhai::AST,
    pub metadata: ScriptMetadata,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn run_id_deps(
        &mut self,
        namespace: &str,
        str_id: &str,
        script_type: ScriptType,
        ast: &rhai::AST,
        scope: &mut rhai::Scope,
    ) -> Result<(), ResourceError> {
        let id_deps = self.engine.call_fn::<rhai::Array>(&mut rhai::Scope::new(), ast, "id_deps", ())?;

        for id_dep in id_deps.into_iter() {
            let mut array = id_dep.cast::<rhai::Array>().into_iter();

            let id = self.interner.get_or_intern(
                array.next().unwrap().cast::<rhai::ImmutableString>().as_str().into(),
                Some(namespace),
            )?;

            let key = array.next().unwrap().cast::<rhai::ImmutableString>();

            match script_type {
                ScriptType::Library => {
                    log::info!("Adding {key} -> {id} into scope of library script '{str_id}'.");
                },
                ScriptType::Tile => {
                    log::info!("Adding {key} -> {id} into scope of tile script '{str_id}'.");
                },
            };

            scope.push_constant(key.as_str(), id);
        }

        Ok(())
    }

    pub fn load_script_file_lib(&mut self, file: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading library script at {}.", file.display());

        let name = file
            .file_stem()
            .ok_or_else(|| ResourceError::NoFileStem(file.to_path_buf()))?
            .to_str()
            .ok_or_else(|| ResourceError::OsStringError(file.to_path_buf()))?;

        let str_id = format!("lib::{}::{}", namespace, name);
        let mut scope = rhai::Scope::new();

        {
            let ast = self.engine.compile_file(file.to_path_buf())?;
            self.run_id_deps(namespace, &str_id, ScriptType::Library, &ast, &mut scope)?;
        }

        let ast = self.engine.compile_file_with_scope(&scope, file.to_path_buf())?;
        let module = rhai::Module::eval_ast_as_new(rhai::Scope::new(), &ast, &self.engine);

        match module {
            Ok(module) => {
                self.engine.register_static_module(str_id.clone(), module.into());
            },
            Err(err) => {
                log::error!("Could not register library script {str_id}!");
                return Err(err.into());
            },
        }

        log::info!("Registered library script with id '{str_id}'!");

        Ok(())
    }

    pub fn load_script_file_tile(&mut self, file: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading tile script at {}.", file.display());

        let mut scope = rhai::Scope::new();

        let str_id = {
            let ast = self.engine.compile_file(file.to_path_buf())?;
            let str_id = self
                .engine
                .call_fn::<rhai::ImmutableString>(&mut rhai::Scope::new(), &ast, "script_id", ())?
                .to_string();
            self.run_id_deps(namespace, &str_id, ScriptType::Tile, &ast, &mut scope)?;

            str_id
        };

        let id = ScriptId(self.interner.get_or_intern(str_id.as_str().into(), Some(namespace))?);
        let ast = self.engine.compile_file_with_scope(&scope, file.to_path_buf())?;

        self.scripts.insert(
            id,
            ScriptData {
                ast,
                metadata: ScriptMetadata {
                    str_id: str_id.clone(),
                },
            },
        );

        log::info!("Registered tile script with Id '{str_id}'!");

        Ok(())
    }

    pub fn load_script_files(&mut self, dir: &Path, namespace: &str) {
        let path = dir.join("scripts");

        {
            let path = path.join("lib");

            for file in read_recursively(&path, SCRIPT_EXTS) {
                match self.load_script_file_lib(&file, namespace) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        }
        {
            let path = path.join("src");

            {
                let path = path.join("tile");

                for file in read_recursively(&path, SCRIPT_EXTS) {
                    match self.load_script_file_tile(&file, namespace) {
                        Ok(_) => {},
                        Err(err) => err.log_err(),
                    }
                }
            }
        }
    }
}
