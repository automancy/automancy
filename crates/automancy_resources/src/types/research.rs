use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_defs::{
    id::{Id, ModelId, TileId},
    parse_ids, parse_item_stacks,
    stack::{ItemAmount, ItemStack},
};
use petgraph::visit::IntoNodeReferences;
use serde::Deserialize;

use crate::{
    RON_EXT, ResourceManager,
    data::{DataMap, DataMapRaw},
    load_recursively,
    types::IconMode,
};

#[derive(Debug, Clone)]
pub struct ResearchDef {
    pub id: Id,
    pub icon: ModelId,
    pub icon_mode: IconMode,
    pub unlocks: Vec<TileId>,
    pub depends_on: Option<Id>,
    pub name: Id,
    pub description: Id,
    pub completed_description: Id,
    pub required_items: Option<Vec<ItemStack>>,
    pub attached_puzzle: Option<(Id, DataMap)>,
}

#[derive(Debug, Deserialize)]
struct Raw {
    id: String,
    icon: String,
    icon_mode: IconMode,
    unlocks: Vec<String>,
    depends_on: Option<String>,
    name: String,
    description: String,
    completed_description: String,
    required_items: Option<Vec<(String, ItemAmount)>>,
    attached_puzzle: Option<(String, DataMapRaw)>,
}

impl ResourceManager {
    fn load_research(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading research entry at: {file:?}");

        let v = ron::from_str::<Raw>(&read_to_string(file)?)?;

        let id = Id::parse(&v.id, &mut self.interner, Some(namespace)).unwrap();

        let unlocks: Vec<Id> =
            parse_ids(v.unlocks.into_iter(), &mut self.interner, Some(namespace));

        let icon = Id::parse(&v.icon, &mut self.interner, Some(namespace)).unwrap();

        let depends_on = v
            .depends_on
            .map(|v| Id::parse(&v, &mut self.interner, Some(namespace)).unwrap());

        let name = Id::parse(&v.name, &mut self.interner, Some(namespace)).unwrap();

        let description = Id::parse(&v.description, &mut self.interner, Some(namespace)).unwrap();

        let completed_description = Id::parse(
            &v.completed_description,
            &mut self.interner,
            Some(namespace),
        )
        .unwrap();

        let required_items = v
            .required_items
            .map(|v| parse_item_stacks(v.into_iter(), &mut self.interner, Some(namespace)));

        let attached_puzzle = v.attached_puzzle.map(|(id, data)| {
            (
                Id::parse(&id, &mut self.interner, Some(namespace)).unwrap(),
                data.intern_to_data(&mut self.interner, Some(namespace)),
            )
        });
        let icon_mode = v.icon_mode;

        let index = self.registry.researches.add_node(ResearchDef {
            id,
            unlocks: unlocks.into_iter().map(TileId).collect(),
            depends_on,
            icon: ModelId(icon),
            name,
            description,
            completed_description,
            required_items,
            attached_puzzle,
            icon_mode,
        });

        self.registry.researches_id_map.insert(id, index);

        for unlock in &self.registry.researches.node_weight(index).unwrap().unlocks {
            if self
                .registry
                .researches_unlock_map
                .insert(*unlock, index)
                .is_some()
            {
                log::warn!(
                    "Unlock for {:?} is overritten by {:?}!",
                    self.interner.resolve(**unlock),
                    self.interner.resolve(id)
                )
            }
        }

        Ok(())
    }

    pub fn load_researches(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let items = dir.join("researches");

        for file in load_recursively(&items, OsStr::new(RON_EXT)) {
            self.load_research(&file, namespace)?;
        }

        Ok(())
    }

    pub fn get_research(&self, id: Id) -> Option<&ResearchDef> {
        self.registry
            .researches_id_map
            .get(&id)
            .and_then(|i| self.registry.researches.node_weight(*i))
    }

    pub fn get_research_by_unlock(&self, id: TileId) -> Option<&ResearchDef> {
        self.registry
            .researches_unlock_map
            .get(&id)
            .and_then(|i| self.registry.researches.node_weight(*i))
    }

    pub fn compile_researches(&mut self) {
        for (this, research) in self.registry.researches.clone().node_references() {
            if let Some(prev) = &research.depends_on {
                if let Some(prev) = self.registry.researches_id_map.get(prev).cloned() {
                    self.registry.researches.add_edge(prev, this, ());
                }
            }
        }
    }
}
