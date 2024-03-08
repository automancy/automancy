use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::Path;

use rhai::{Dynamic, ImmutableString, Scope};

use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::data::{Data, DataMap};
use crate::{ResourceManager, FUNCTION_EXT};

#[derive(Default, Debug, Clone)]
pub struct RhaiDataMap(BTreeMap<Id, Dynamic>);

impl RhaiDataMap {
    pub fn rhai_get(&mut self, id: Id) -> Dynamic {
        if let Some(v) = self.0.get(&id).cloned() {
            v.cast::<Data>().rhai_value()
        } else {
            Dynamic::UNIT
        }
    }

    pub fn rhai_set(&mut self, id: Id, v: Dynamic) {
        if let Some(v) = Data::from_rhai(v) {
            self.0.insert(id, Dynamic::from(v));
        }
    }

    pub fn rhai_get_or_new_inventory(&mut self, id: Id) -> Dynamic {
        self.0
            .entry(id)
            .or_insert_with(|| Dynamic::from(Data::Inventory(Default::default())));
        self.rhai_get(id)
    }

    pub fn get(&self, id: Id) -> Option<Data> {
        self.0.get(&id).map(|v| v.clone_cast())
    }

    pub fn set(&mut self, id: Id, data: Data) {
        self.0.insert(id, Dynamic::from(data));
    }

    pub fn remove(&mut self, id: Id) {
        self.0.remove(&id);
    }

    pub fn to_data_map(self) -> DataMap {
        self.0
            .into_iter()
            .map(|(k, mut v)| (k, v.take().cast::<Data>()))
            .collect::<BTreeMap<Id, Data>>()
            .into()
    }

    pub fn from_data_map(map: DataMap) -> Self {
        RhaiDataMap(
            map.into_inner()
                .into_iter()
                .map(|(k, v)| (k, Dynamic::from(v)))
                .collect(),
        )
    }
}

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

                log::info!("Registered function with id {str_id} ({id:?})");

                self.functions.insert(id, (ast, scope, str_id));
            }
        }

        Ok(())
    }
}
