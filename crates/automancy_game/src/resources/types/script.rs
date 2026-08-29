use std::path::Path;

use automancy_data::id::{MUTABLE_GLOBAL_INTERNER, ScriptId};

use crate::resources::{MutableResourceManager, RHAI_EXT, ResourceError, SCRIPTS_PATH, UNIT_TESTS_ENABLED, read_recursively};

#[derive(Debug)]
pub enum ScriptType {
    Library,
    Tile,
}

#[derive(Debug, Clone, Copy)]
pub struct ScriptMetadata {
    pub str_id: &'static str,
}

#[derive(Debug, Clone)]
pub struct RhaiScriptData {
    pub ast: rhai::AST,
    pub metadata: ScriptMetadata,
}

fn parse_path_name(path: &Path) -> Result<String, ResourceError> {
    // strip extension
    let path = path.with_extension("");

    let mut vec = Vec::new();

    for component in path.components() {
        let s = match component.as_os_str().to_str() {
            Some(v) => v,
            None => return Err(ResourceError::OsStringError(path)),
        };

        if s.contains(".") {
            return Err(ResourceError::PathContainsDot(path));
        }

        vec.push(s);
    }

    Ok(vec.join("/"))
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn run_id_deps(
        namespace: &str,
        rhai: &mut rhai::Engine,
        str_id: &str,
        script_type: ScriptType,
        ast: &rhai::AST,
        scope: &mut rhai::Scope,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        let id_deps = rhai.call_fn::<rhai::Array>(&mut rhai::Scope::new(), ast, "id_deps", ())?;

        for id_dep in id_deps.into_iter() {
            let mut array = id_dep.cast::<rhai::Array>().into_iter();

            let id = MUTABLE_GLOBAL_INTERNER.with_borrow_mut(|interner| {
                interner
                    .as_mut()
                    .unwrap()
                    .get_or_intern(&array.next().unwrap().cast::<rhai::ImmutableString>(), Some(namespace))
                    .unwrap()
            });

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

    pub fn load_script_file_lib_rhai(
        path: &Path,
        namespace: &str,
        name: &str,
        rhai: &mut rhai::Engine,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        let str_id = format!("lib::{namespace}::{name}");

        let mut scope = rhai::Scope::new();

        {
            let ast = rhai.compile_file(path.to_path_buf())?;
            MutableResourceManager::run_id_deps(namespace, rhai, &str_id, ScriptType::Library, &ast, &mut scope)?;
        }

        let ast = rhai.compile_file_with_scope(&scope, path.to_path_buf())?;
        let module = rhai::Module::eval_ast_as_new(rhai::Scope::new(), &ast, rhai)?;
        rhai.register_static_module(str_id, module.into());

        Ok(())
    }

    pub fn load_script_file_tile_rhai(
        path: &Path,
        namespace: &str,
        name: &str,
        rhai: &mut rhai::Engine,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        let str_id = format!("{namespace}:{name}");

        let mut scope = rhai::Scope::new();

        {
            let ast = rhai.compile_file(path.to_path_buf())?;
            MutableResourceManager::run_id_deps(namespace, rhai, &str_id, ScriptType::Tile, &ast, &mut scope)?;
        }

        let ast = rhai.compile_file_with_scope(&scope, path.to_path_buf())?;

        MutableResourceManager::with_interner(|resource_man, interner| {
            let id = ScriptId(interner.get_or_intern(&str_id, Some(namespace)).unwrap());
            resource_man.rhai_scripts.insert(
                id,
                RhaiScriptData {
                    ast,
                    metadata: ScriptMetadata {
                        str_id: str_id.leak(),
                    },
                },
            );
        });

        Ok(())
    }

    fn inner_load_script_files(path: &Path, namespace: &str, rhai: &mut rhai::Engine) -> Result<(), ResourceError> {
        {
            let lib_path = path.join("lib");

            for entry in read_recursively(&lib_path, [RHAI_EXT]) {
                let path = entry.path();

                log::info!("Loading library script at {}.", path.display());

                let name = match parse_path_name(path.trim_prefix(&lib_path)) {
                    Ok(name) => name,
                    Err(err) => {
                        log::error!("Couldn't get library script name from path!");
                        err.log_err();
                        continue;
                    },
                };

                let result = if path
                    .extension()
                    .is_some_and(|ext| [RHAI_EXT].into_iter().any(|valid| ext.eq_ignore_ascii_case(valid)))
                {
                    Self::load_script_file_lib_rhai(path, namespace, &name, rhai).map_err(ResourceError::RhaiEvalError)
                } else {
                    continue;
                };

                match result {
                    Ok(()) => {
                        log::info!("Registered library script with Id '{namespace}:{name}'!");
                    },
                    Err(err) => {
                        log::error!("Couldn't register library script with Id '{namespace}:{name}'!");
                        err.log_err()
                    },
                }
            }
        }

        {
            let path = path.join("src");

            {
                let tile_path = path.join("tile");

                for entry in read_recursively(&tile_path, [RHAI_EXT]) {
                    let path = entry.path();

                    log::info!("Loading tile script at {}.", path.display());

                    let name = match parse_path_name(path.trim_prefix(&tile_path)) {
                        Ok(name) => format!("tile/{name}"),
                        Err(err) => {
                            log::error!("Couldn't get tile script name from path!");
                            err.log_err();
                            continue;
                        },
                    };

                    let result = if path
                        .extension()
                        .is_some_and(|ext| [RHAI_EXT].into_iter().any(|valid| ext.eq_ignore_ascii_case(valid)))
                    {
                        Self::load_script_file_tile_rhai(path, namespace, &name, rhai).map_err(ResourceError::RhaiEvalError)
                    } else {
                        continue;
                    };

                    match result {
                        Ok(()) => {
                            log::info!("Registered tile script with Id '{namespace}:{name}'!");
                        },
                        Err(err) => {
                            log::error!("Couldn't register tile script with Id '{namespace}:{name}'!");
                            err.log_err()
                        },
                    }
                }
            }
        }

        if UNIT_TESTS_ENABLED.get() {
            let test_path = path.join("test");

            for entry in read_recursively(&test_path, [RHAI_EXT]) {
                let path = entry.path();

                log::info!("Running test script at {}.", path.display());

                let name = match parse_path_name(path.trim_prefix(&test_path)) {
                    Ok(name) => format!("test/{name}"),
                    Err(err) => {
                        log::error!("Couldn't get test script name from path!");
                        err.log_err();
                        continue;
                    },
                };

                // unimplemented
            }
        }

        Ok(())
    }

    pub fn load_script_files(dir: &Path, namespace: &str, rhai: &mut rhai::Engine) {
        let path = dir.join(SCRIPTS_PATH);

        Self::inner_load_script_files(&path, namespace, rhai).unwrap();
    }
}
