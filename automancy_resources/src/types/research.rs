use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::{Deserialize, Serialize};

use automancy_defs::graph::visit::IntoNodeReferences;
use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::data::stack::{ItemAmount, ItemStack};
use crate::data::DataMapRaw;
use crate::types::function::RhaiDataMap;
use crate::types::IconMode;
use crate::{load_recursively, ResourceError, ResourceManager, RON_EXT};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResearchRaw {
    id: IdRaw,
    icon: IdRaw,
    icon_mode: IconMode,
    unlocks: Vec<IdRaw>,
    depends_on: Option<IdRaw>,
    name: IdRaw,
    description: IdRaw,
    completed_description: IdRaw,
    required_items: Option<Vec<(IdRaw, ItemAmount)>>,
    attached_puzzle: Option<(IdRaw, DataMapRaw)>,
}

#[derive(Debug, Clone)]
pub struct Research {
    pub id: Id,
    pub icon: Id,
    pub icon_mode: IconMode,
    pub unlocks: Vec<Id>,
    pub depends_on: Option<Id>,
    pub name: Id,
    pub description: Id,
    pub completed_description: Id,
    pub required_items: Option<Vec<ItemStack>>,
    pub attached_puzzle: Option<(Id, RhaiDataMap)>,
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
        let depends_on = research.depends_on.map(|id| id.to_id(&mut self.interner));
        let name = research.name.to_id(&mut self.interner);
        let description = research.description.to_id(&mut self.interner);
        let completed_description = research.completed_description.to_id(&mut self.interner);
        let required_items = match research.required_items.map(|v| {
            v.into_iter()
                .map(|(id, amount)| {
                    Ok(ItemStack {
                        item: *self
                            .registry
                            .items
                            .get(&id.to_id(&mut self.interner))
                            .ok_or(ResourceError::ItemNotFound)?,
                        amount,
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        }) {
            Some(Err(e)) => return Err(e),
            Some(Ok(v)) => Some(v),
            _ => None,
        };
        let attached_puzzle = research.attached_puzzle.map(|(id, data)| {
            (
                id.to_id(&mut self.interner),
                RhaiDataMap::from_data_map(data.intern_to_data(&mut self.interner)),
            )
        });
        let icon_mode = research.icon_mode;

        let index = self.registry.researches.add_node(Research {
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
        for (this, research) in self.registry.researches.clone().node_references() {
            if let Some(prev) = &research.depends_on {
                if let Some(prev) = self.registry.researches_id_map.get(prev).cloned() {
                    self.registry.researches.add_edge(prev, this, ());
                }
            }
        }
    }

    pub fn get_puzzle_model(&self, maybe_item: Id) -> Id {
        if self.registry.items.contains_key(&maybe_item) {
            self.registry.items[&maybe_item].model
        } else {
            self.registry.model_ids.puzzle_space
        }
    }
}
