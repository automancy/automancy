use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};

use boa_engine::syntax::ast::node::StatementList;
use boa_engine::Context;
use serde::Deserialize;

use crate::resource::{ResourceManager, JSON_EXT};
use crate::util::id::IdRaw;

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionRaw {
    pub id: IdRaw,
    pub tick: Option<String>,
    pub transaction: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub tick: Option<StatementList>,
    pub transaction: Option<StatementList>,
}

fn load_function(context: &mut Context, file: Option<PathBuf>) -> Option<StatementList> {
    if let Some(file) = file {
        log::info!("loading function at {file:?}");

        let ast = context.parse(&read_to_string(file).unwrap()).unwrap();
        Some(ast)
    } else {
        None
    }
}

impl ResourceManager {
    pub fn load_functions(&mut self, dir: &Path) -> Option<()> {
        let functions = dir.join("functions");
        let functions = read_dir(functions).ok()?;

        let mut context = Context::default();

        functions
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .map(|file| {
                log::info!("loading function at {file:?}");

                let function: FunctionRaw = serde_json::from_str(
                    &read_to_string(&file)
                        .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
                )
                .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}"));

                let tick = function.tick.map(|v| file.parent().unwrap().join(v));
                let transaction = function.transaction.map(|v| file.parent().unwrap().join(v));

                (function.id, (tick, transaction))
            })
            .for_each(|(id, (tick, transaction))| {
                let id = id.to_id(&mut self.interner);

                let tick = load_function(&mut context, tick);
                let transaction = load_function(&mut context, transaction);

                self.functions.insert(id, Function { tick, transaction });
            });

        Some(())
    }
}
