use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::Path;

use rhai::{ImmutableString, Scope};

use automancy_defs::id::IdRaw;
use automancy_defs::log;

use crate::{ResourceManager, FUNCTION_EXT};

impl ResourceManager {
    pub fn load_functions(&mut self, dir: &Path) -> Option<()> {
        let functions = dir.join("functions");
        let functions = read_dir(functions).ok()?;

        let mut scope = Scope::new();

        functions
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(FUNCTION_EXT)))
            .for_each(|file| {
                log::info!("loading function at {file:?}");
                let ast = self.engine.compile_file(file).unwrap();

                let str_id = self
                    .engine
                    .call_fn::<ImmutableString>(&mut scope, &ast, "function_id", ())
                    .unwrap();
                let id = self
                    .interner
                    .get_or_intern(IdRaw::parse(&str_id).to_string());

                self.functions.insert(id, ast);

                log::info!("registered function with id {str_id} ({id:?})");
            });

        Some(())
    }
}
