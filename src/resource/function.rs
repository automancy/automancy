use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;
use std::sync::Arc;

use rune::runtime::RuntimeContext;
use rune::termcolor::StandardStream;
use rune::{Context, Diagnostics, Module, Source, Sources, Unit};
use serde::Deserialize;

use crate::game::item::ItemStack;
use crate::game::tile::coord::TileCoord;
use crate::game::tile::entity::TileEntity;
use crate::resource::item::{id_eq_or_of_tag, Item};
use crate::resource::script::{Instructions, Script};
use crate::resource::tag::Tag;
use crate::resource::tile::{Tile, TileType};
use crate::resource::{Registry, ResourceManager, JSON_EXT};
use crate::util::id::{Id, IdRaw};

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
        module.ty::<Id>().unwrap();

        module.ty::<Tile>().unwrap();
        module.ty::<TileType>().unwrap();

        module.ty::<Script>().unwrap();
        module.ty::<Instructions>().unwrap();

        module.ty::<Tag>().unwrap();

        module.ty::<Item>().unwrap();

        module.ty::<ItemStack>().unwrap();

        module.ty::<Registry>().unwrap();
        module.inst_fn("get_tile", Registry::get_tile).unwrap();
        module.inst_fn("get_script", Registry::get_script).unwrap();
        module.inst_fn("get_tag", Registry::get_tag).unwrap();
        module.inst_fn("get_item", Registry::get_item).unwrap();

        module
            .function(&["id_eq_or_of_tag"], id_eq_or_of_tag)
            .unwrap();
        TileCoord::install(&mut module).unwrap();
        TileEntity::install(&mut module).unwrap();

        let mut diagnostics = Diagnostics::new();

        let mut context = Context::with_default_modules().unwrap();
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
                    &read_to_string(&file)
                        .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
                )
                .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}"));

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
                    let mut writer = StandardStream::stderr(Default::default());

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
