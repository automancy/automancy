use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_data::{
    game::{
        generic::{DataMap, deserialize::DataMapStr},
        inventory::{ItemStack, deserialize::ItemStackStr},
    },
    id::{
        Id, ModelId, TileId,
        deserialize::{StrId, StrIdExt},
        parse::{parse_ids, parse_item_stacks},
    },
};
use petgraph::visit::IntoNodeReferences;
use serde::Deserialize;

use crate::{
    persistent,
    resources::{RON_EXT, ResourceManager, load_recursively, types::IconMode},
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
    id: StrId,
    icon: StrId,
    icon_mode: IconMode,
    unlocks: Vec<StrId>,
    depends_on: Option<StrId>,
    name: StrId,
    description: StrId,
    completed_description: StrId,

    required_items: Option<Vec<ItemStackStr>>,
    attached_puzzle: Option<(StrId, DataMapStr)>,
}

impl ResourceManager {
    fn load_research(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading research entry at: {file:?}");

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = v.id.into_id(&mut self.interner, Some(namespace))?;

        let unlocks = parse_ids(v.unlocks.into_iter(), &mut self.interner, Some(namespace))
            .map(|v| v.map(TileId))
            .try_collect()?;

        let icon = v.icon.into_id(&mut self.interner, Some(namespace))?;

        let depends_on = v.depends_on.into_id(&mut self.interner, Some(namespace))?;

        let name = v.name.into_id(&mut self.interner, Some(namespace))?;

        let description = v.description.into_id(&mut self.interner, Some(namespace))?;

        let completed_description = v.completed_description.into_id(&mut self.interner, Some(namespace))?;

        let required_items = match v.required_items {
            Some(v) => Some(parse_item_stacks(v.into_iter(), &mut self.interner, Some(namespace)).try_collect()?),
            None => None,
        };

        let attached_puzzle = match v.attached_puzzle {
            Some((id, data)) => Some((
                id.into_id(&mut self.interner, Some(namespace))?,
                data.into_data(&mut self.interner, Some(namespace))?,
            )),
            None => None,
        };
        let icon_mode = v.icon_mode;

        let index = self.registry.researche_defs.add_node(ResearchDef {
            id,
            unlocks,
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

        for unlock in &self.registry.researche_defs.node_weight(index).unwrap().unlocks {
            if self.registry.researches_unlock_map.insert(*unlock, index).is_some() {
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
            .and_then(|i| self.registry.researche_defs.node_weight(*i))
    }

    pub fn get_research_by_unlock(&self, id: TileId) -> Option<&ResearchDef> {
        self.registry
            .researches_unlock_map
            .get(&id)
            .and_then(|i| self.registry.researche_defs.node_weight(*i))
    }

    pub fn compile_researches(&mut self) {
        for (this, research) in self.registry.researche_defs.clone().node_references() {
            if let Some(prev) = &research.depends_on
                && let Some(prev) = self.registry.researches_id_map.get(prev).cloned()
            {
                self.registry.researche_defs.add_edge(prev, this, ());
            }
        }
    }

    pub fn is_research_unlocked(&self, research: Id, game_data: &mut DataMap) -> bool {
        let unlocked = game_data.set_id_mut(self.registry.data_ids.unlocked_researches);

        if unlocked.contains(&research) {
            return true;
        }

        false
    }

    pub fn should_category_show(&self, category: Id, game_data: &mut DataMap) -> bool {
        let Some(category) = self.registry.categorie_defs.get(&category) else {
            return false;
        };

        let Some(tiles) = self.get_tiles_by_category(category.id) else {
            return false;
        };

        if tiles.iter().any(|id| {
            self.registry.tile_defs[id]
                .data
                .bool_or_default(self.registry.data_ids.default_tile, false)
        }) {
            return true;
        }

        let Some(researches) = self.get_researches_by_category(category.id) else {
            return false;
        };

        let unlocked = game_data.set_id_mut(self.registry.data_ids.unlocked_researches);

        for research in researches {
            if unlocked.contains(&research) {
                return true;
            }
        }

        false
    }
}
