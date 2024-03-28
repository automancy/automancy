use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::{Deserialize, Serialize};

use automancy_defs::graph::visit::IntoNodeReferences;
use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::data::stack::{ItemAmount, ItemStack};
use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResearchRaw {
    id: IdRaw,
    icon: IdRaw,
    unlocks: Vec<IdRaw>,
    next_researches: Option<Vec<IdRaw>>,
    name: IdRaw,
    description: IdRaw,
    required_items: Option<Vec<(IdRaw, ItemAmount)>>,
    attached_puzzle: Option<IdRaw>,
}

#[derive(Debug, Clone)]
pub struct Research {
    pub id: Id,
    pub icon: Id,
    pub unlocks: Vec<Id>,
    pub next_researches: Option<Vec<Id>>,
    pub name: Id,
    pub description: Id,
    pub required_items: Option<Vec<ItemStack>>,
    pub attached_puzzle: Option<Id>,
}

impl ResourceManager {
    fn load_research(&mut self, file: &Path) -> anyhow::Result<()> {
        log::info!("Loading research entry at: {file:?}");

        let research: ResearchRaw = ron::from_str(&read_to_string(file)?)?;

        let id = research.id.to_id(&mut self.interner);
        let unlocks = research
            .unlocks
            .into_iter()
            .map(|id| id.to_id(&mut self.interner))
            .collect();
        let icon = research.icon.to_id(&mut self.interner);
        let next = research
            .next_researches
            .map(|v| v.iter().map(|id| id.to_id(&mut self.interner)).collect());
        let name = research.name.to_id(&mut self.interner);
        let description = research.description.to_id(&mut self.interner);
        let required_items = research.required_items.map(|v| {
            v.into_iter()
                .map(|(id, amount)| ItemStack {
                    item: self.registry.items[&id.to_id(&mut self.interner)],
                    amount,
                })
                .collect()
        });
        let attached_puzzle = research
            .attached_puzzle
            .map(|id| id.to_id(&mut self.interner));

        let index = self.registry.researches.add_node(Research {
            id,
            unlocks,
            next_researches: next,
            icon,
            name,
            description,
            required_items,
            attached_puzzle,
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

    pub fn load_researches(&mut self, dir: &Path) -> anyhow::Result<()> {
        let items = dir.join("researches");

        for file in load_recursively(&items, OsStr::new(RON_EXT)) {
            self.load_research(&file)?;
        }

        Ok(())
    }

    pub fn get_research(&self, id: Id) -> Option<&Research> {
        self.registry
            .researches_id_map
            .get(&id)
            .and_then(|i| self.registry.researches.node_weight(*i))
    }

    pub fn get_research_by_unlock(&self, id: Id) -> Option<&Research> {
        self.registry
            .researches_unlock_map
            .get(&id)
            .and_then(|i| self.registry.researches.node_weight(*i))
    }

    pub fn compile_researches(&mut self) {
        for (index, research) in self.registry.researches.clone().node_references() {
            if let Some(next) = &research.next_researches {
                for id in next {
                    if let Some(next_index) = self.registry.researches_id_map.get(id).cloned() {
                        self.registry.researches.add_edge(index, next_index, ());
                    }
                }
            }
        }
    }
}
