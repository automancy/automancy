use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::Deserialize;

use automancy_defs::{graph::visit::IntoNodeReferences, id::Id, parse_item_stacks};
use automancy_defs::{
    parse_ids,
    stack::{ItemAmount, ItemStack},
};

use crate::data::{DataMap, DataMapRaw};
use crate::types::IconMode;
use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Debug, Clone)]
pub struct ResearchDef {
    pub id: Id,
    pub icon: Id,
    pub icon_mode: IconMode,
    pub unlocks: Vec<Id>,
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

        let unlocks = parse_ids(v.unlocks.into_iter(), &mut self.interner, Some(namespace));

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
            unlocks,
            depends_on,
            icon,
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
                    self.interner.resolve(*unlock),
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

    pub fn get_research_by_unlock(&self, id: Id) -> Option<&ResearchDef> {
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
