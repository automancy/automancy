use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use rhai::{Dynamic, ImmutableString, Scope};

use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::data::{Data, DataMap};
use crate::{load_recursively, ResourceManager, FUNCTION_EXT};

#[derive(Debug, Clone, Copy)]
pub enum ResultType {
    MakeTransaction,
    MakeExtractRequest,
}

#[derive(Debug, Clone, Copy)]
pub enum TransactionResultType {
    PassOn,
    Proxy,
    Consume,
}

#[derive(Default, Debug, Clone)]
pub struct RhaiDataMap(BTreeMap<Id, Data>);

impl RhaiDataMap {
    pub fn rhai_get(&mut self, id: Id) -> Dynamic {
        if let Some(v) = self.get(id).cloned() {
            v.rhai_value()
        } else {
            Dynamic::UNIT
        }
    }

    pub fn rhai_set(&mut self, id: Id, v: Dynamic) {
        if let Some(v) = Data::from_rhai(v) {
            self.set(id, v);
        }
    }

    pub fn rhai_get_or_new_inventory(&mut self, id: Id) -> Dynamic {
        self.0
            .entry(id)
            .or_insert_with(|| Data::Inventory(Default::default()))
            .clone()
            .rhai_value()
    }

    pub fn get(&self, id: Id) -> Option<&Data> {
        self.0.get(&id)
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut Data> {
        self.0.get_mut(&id)
    }

    pub fn set(&mut self, id: Id, data: Data) {
        self.0.insert(id, data);
    }

    pub fn remove(&mut self, id: Id) {
        self.0.remove(&id);
    }

    pub fn to_data_map(self) -> DataMap {
        self.0.into()
    }

    pub fn from_data_map(data: DataMap) -> Self {
        Self(data.into_inner())
    }
}

impl ResourceManager {
    pub fn load_functions(&mut self, dir: &Path) -> anyhow::Result<()> {
        let functions = dir.join("functions");

        for file in load_recursively(&functions, OsStr::new(FUNCTION_EXT)) {
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

                    let id =
                        IdRaw::parse(v[0].clone().cast::<ImmutableString>().as_str()).to_string();
                    let key = v[1].clone().cast::<ImmutableString>();

                    log::info!("Adding {key} -> {id} into scope of function {str_id}");

                    scope.push_constant(key.as_str(), self.interner.get_or_intern(&id));
                });
            }

            log::info!("Registered function with id {str_id} ({id:?})");

            self.functions.insert(id, (ast, scope, str_id));
        }

        Ok(())
    }
}
