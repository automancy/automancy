use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use rhai::{Dynamic, ImmutableString, Scope};

use automancy_defs::{
    coord::TileCoord,
    id::{Id, IdRaw},
    stack::ItemStack,
};

use crate::{
    data::{Data, DataMap},
    load_recursively, ResourceManager, FUNCTION_EXT,
};

#[derive(Debug, Clone)]
pub enum TileResult {
    MakeTransaction {
        coord: TileCoord,
        source_id: Id,
        source_coord: TileCoord,
        stacks: Vec<ItemStack>,
    },
    MakeExtractRequest {
        coord: TileCoord,
        requested_from_id: Id,
        requested_from_coord: TileCoord,
        on_fail_action: OnFailAction,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum TileTransactionResult {
    PassOn {
        coord: TileCoord,
        stack: ItemStack,
        source_coord: TileCoord,
        root_coord: TileCoord,
        root_id: Id,
    },
    Proxy {
        coord: TileCoord,
        stack: ItemStack,
        source_coord: TileCoord,
        source_id: Id,
        root_coord: TileCoord,
        root_id: Id,
    },
    Consume {
        consumed: ItemStack,
        source_coord: TileCoord,
        root_coord: TileCoord,
    },
}

#[derive(Debug, Copy, Clone)]
pub enum OnFailAction {
    None,
    RemoveTile,
    RemoveAllData,
    RemoveData(Id),
}

#[derive(Default, Debug, Clone)]
pub struct RhaiDataMap(BTreeMap<Id, Data>);

impl RhaiDataMap {
    pub fn rhai_get(&mut self, id: Id) -> Dynamic {
        if let Some(v) = self.get(id).cloned() {
            v.into_dynamic()
        } else {
            Dynamic::UNIT
        }
    }

    pub fn rhai_set(&mut self, id: Id, data: Dynamic) {
        if let Some(data) = Data::from_dynamic(data) {
            self.set(id, data);
        }
    }

    pub fn get_or_new_inventory(&mut self, id: Id) -> Dynamic {
        self.0
            .entry(id)
            .or_insert_with(|| Data::Inventory(Default::default()))
            .clone()
            .into_dynamic()
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
    pub fn load_functions(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let functions = dir.join("functions");

        for file in load_recursively(&functions, OsStr::new(FUNCTION_EXT)) {
            log::info!("Loading function at {file:?}");

            let mut scope = Scope::new();
            let ast = self.engine.compile_file(file)?;

            let raw_id =
                self.engine
                    .call_fn::<ImmutableString>(&mut scope, &ast, "function_id", ())?;
            let raw_id = IdRaw::parse(&raw_id, Some(namespace)).unwrap();
            let str_id = raw_id.to_string();

            let id = raw_id.to_id(&mut self.interner);

            let id_deps = self
                .engine
                .call_fn::<rhai::Array>(&mut scope, &ast, "id_deps", ())?;

            for id_dep in id_deps.into_iter() {
                let v = id_dep.cast::<rhai::Array>();

                let id = IdRaw::parse(
                    v[0].clone().cast::<ImmutableString>().as_str(),
                    Some(namespace),
                )
                .unwrap();

                let key = v[1].clone().cast::<ImmutableString>();

                log::info!("Adding {key} -> {id} into scope of function {str_id}",);

                scope.push_constant(
                    key.as_str(),
                    Id::parse(&id, &mut self.interner, Some(namespace)).unwrap(),
                );
            }

            log::info!("Registered function with id {str_id} ({id:?})");

            self.functions.insert(id, (ast, scope, str_id));
        }

        Ok(())
    }
}
