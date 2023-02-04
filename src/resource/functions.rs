use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;
use std::sync::Arc;

use crate::game::data::TileCoord;
use rune::runtime::RuntimeContext;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Module, Source, Sources, Unit};
use serde::Deserialize;

use crate::resource::{ResourceManager, JSON_EXT};
use crate::util::id::IdRaw;

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionRaw {
    pub id: IdRaw,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub context: Arc<RuntimeContext>,
    pub unit: Arc<Unit>,
}

impl ResourceManager {
    pub fn load_functions(&mut self, dir: &Path) -> Option<()> {
        let functions = dir.join("functions");
        let functions = read_dir(functions).ok()?;

        let mut module = Module::new();
        TileCoord::install(&mut module).unwrap();

        let mut diagnostics = Diagnostics::new();

        let mut context = rune::Context::with_default_modules().unwrap();
        context.install(&module).unwrap();

        let runtime = Arc::new(context.runtime());

        functions
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(JSON_EXT)))
            .map(|file| {
                log::info!("loading function at {file:?}");

                let function: FunctionRaw = serde_json::from_str(
                    &read_to_string(&file).unwrap_or_else(|_| panic!("error loading {file:?}")),
                )
                .unwrap_or_else(|_| panic!("error loading {file:?}"));

                (function.id, file.parent().unwrap().join(function.file))
            })
            .for_each(|(id, rune)| {
                log::info!("loading rune script at {rune:?}");

                let mut sources = Sources::new();
                sources.insert(Source::new(id.to_string(), read_to_string(rune).unwrap()));

                let unit = rune::prepare(&mut sources)
                    .with_context(&context)
                    .with_diagnostics(&mut diagnostics)
                    .build();

                if !diagnostics.is_empty() {
                    let mut writer = StandardStream::stderr(ColorChoice::Always);

                    diagnostics.emit(&mut writer, &sources).unwrap();
                }

                let unit = unit.unwrap();

                let id = id.to_id(&mut self.interner);

                self.functions.insert(
                    id,
                    Function {
                        context: runtime.clone(),
                        unit: Arc::new(unit),
                    },
                );
            });

        Some(())
    }
}
