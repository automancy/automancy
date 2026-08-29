use std::{fs::read_to_string, path::Path};

use automancy_data::{
    game::{
        generic::{DataMap, deserialize::DataMapStr},
        inventory::{ItemStack, deserialize::ItemStackStr},
    },
    id::{
        CategoryId, IdInterner, ResearchId, ResearchTranslateId, ScriptId, TileId,
        deserialize::StrId,
        parse::{parse_ids, parse_item_stacks},
    },
    id_map::IdMap,
    rendering::{GenericModel, deserialize::StrGenericModel},
};
use petgraph::{graph::NodeIndex, visit::IntoNodeReferences};
use serde::Deserialize;

use crate::{
    persistent,
    resources::{MutableResourceManager, RON_EXTS, ResourceError, ResourceManager, read_recursively},
};

#[derive(Debug, Clone)]
pub struct ResearchDef {
    pub id: ResearchId,
    pub icon: GenericModel,
    pub unlocks: Vec<TileId>,
    pub depends_on: ResearchId,
    pub name: ResearchTranslateId,
    pub description: ResearchTranslateId,
    pub completed_description: ResearchTranslateId,
    pub required_items: Option<Vec<ItemStack>>,
    pub attached_puzzle: Option<(ScriptId, DataMap)>,
}

#[derive(Debug, Deserialize)]
struct Raw {
    id: StrId,
    icon: StrGenericModel,
    unlocks: Vec<StrId>,
    depends_on: Option<StrId>,
    name: StrId,
    description: StrId,
    completed_description: StrId,

    required_items: Option<Vec<ItemStackStr>>,
    attached_puzzle: Option<(StrId, DataMapStr)>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_research_file(&mut self, interner: &mut IdInterner, path: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading research entry at: {}.", path.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(path)?)?;

        let id = ResearchId(interner.get_or_intern(&v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "research",
                file: path.to_path_buf(),
                name: interner.resolve(*id).unwrap().to_string(),
            });
        }
        let unlocks = parse_ids(v.unlocks.into_iter(), interner, Some(namespace))
            .map(|v| v.map(TileId))
            .try_collect()?;
        let icon = v.icon.into_icon(interner, Some(namespace))?;
        let depends_on = ResearchId(interner.get_or_intern_opt(v.depends_on.as_deref(), Some(namespace))?);
        let name = ResearchTranslateId(interner.get_or_intern(&v.name, Some(namespace))?);
        let description = ResearchTranslateId(interner.get_or_intern(&v.description, Some(namespace))?);
        let completed_description = ResearchTranslateId(interner.get_or_intern(&v.completed_description, Some(namespace))?);
        let required_items = match v.required_items {
            Some(v) => Some(parse_item_stacks(v.into_iter(), interner, Some(namespace)).try_collect()?),
            None => None,
        };
        let attached_puzzle = match v.attached_puzzle {
            Some((id, data)) => Some((
                ScriptId(interner.get_or_intern(&id, Some(namespace))?),
                data.into_data(interner, Some(namespace))?,
            )),
            None => None,
        };

        let index = self.registry.research_defs.add_node(ResearchDef {
            id,
            unlocks,
            depends_on,
            icon,
            name,
            description,
            completed_description,
            required_items,
            attached_puzzle,
        });
        self.registry.research_id_map.insert(id, index);

        Ok(())
    }

    pub fn load_research_files(dir: &Path, namespace: &str) {
        MutableResourceManager::with_interner(|resource_man, interner| {
            let path = dir.join("researches");

            for entry in read_recursively(&path, RON_EXTS) {
                match resource_man.load_research_file(interner, entry.path(), namespace) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
    }

    pub fn compile_researches(&mut self) -> IdMap<TileId, NodeIndex> {
        for (this, research) in self.registry.research_defs.clone().node_references() {
            if let Some(prev) = self.registry.research_id_map.get(&research.depends_on).cloned() {
                self.registry.research_defs.add_edge(prev, this, ());
            }
        }

        let mut research_unlock_map = IdMap::new();

        for (&id, &index) in &self.registry.research_id_map {
            for &unlock in &self.registry.research_defs.node_weight(index).unwrap().unlocks {
                if let Some(old_index) = research_unlock_map.insert(unlock, index) {
                    let old_id = self.registry.research_defs.node_weight(old_index).unwrap().id;

                    log::warn!("Unlock for {unlock} is overwritten by {id}! (was {old_id})")
                }
            }
        }

        research_unlock_map
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl ResourceManager {
    pub fn get_research(&self, id: ResearchId) -> Option<&ResearchDef> {
        self.registry
            .research_id_map
            .get(&id)
            .and_then(|i| self.registry.research_defs.node_weight(*i))
    }

    pub fn get_research_by_unlock(&self, id: TileId) -> Option<&ResearchDef> {
        self.registry
            .research_unlock_map
            .get(&id)
            .and_then(|i| self.registry.research_defs.node_weight(*i))
    }

    pub fn is_research_unlocked(&self, research: ResearchId, map_data: &DataMap) -> bool {
        #[cfg(debug_assertions)]
        if map_data.bool(self.registry.data_ids.debug_unlock_everything).copied() == Some(true) {
            return true;
        }

        if map_data.contains_research_id(self.registry.data_ids.unlocked_researches, research) {
            return true;
        }

        false
    }

    pub fn should_category_show(&self, category: CategoryId, map_data: &DataMap) -> bool {
        let Some(category) = self.registry.category_defs.get(&category) else {
            return false;
        };

        let Some(tiles) = self.get_tiles_by_category(category.id) else {
            return false;
        };

        #[cfg(debug_assertions)]
        if map_data.bool(self.registry.data_ids.debug_unlock_everything).copied() == Some(true) {
            return true;
        }

        if tiles
            .iter()
            .any(|id| self.registry.tile_defs[id].data.bool(self.registry.data_ids.default_tile).copied() == Some(true))
        {
            return true;
        }

        let Some(researches) = self.get_researches_by_category(category.id) else {
            return false;
        };

        if let Some(unlocked) = map_data.set_research_id(self.registry.data_ids.unlocked_researches) {
            for research in researches {
                if unlocked.contains(&research) {
                    return true;
                }
            }
        }

        false
    }
}
