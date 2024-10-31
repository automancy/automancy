use crate::{load_recursively, ResourceManager, RON_EXT};
use automancy_defs::id::{Id, ModelId, TileId};
use hashbrown::HashMap;
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use super::IconMode;

#[derive(Debug, Clone, Copy)]
pub struct CategoryDef {
    pub id: Id,
    pub ord: i32,
    pub icon: Id,
    pub icon_mode: IconMode,
    pub item: Option<Id>,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: String,
    pub ord: i32,
    pub icon: String,
    pub icon_mode: IconMode,
    pub item: Option<String>,
}

impl ResourceManager {
    fn load_category(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading category at: {file:?}");

        let v = ron::from_str::<Raw>(&read_to_string(file)?)?;

        let id = Id::parse(&v.id, &mut self.interner, Some(namespace)).unwrap();
        let ord = v.ord;
        let icon = Id::parse(&v.icon, &mut self.interner, Some(namespace)).unwrap();
        let icon_mode = v.icon_mode;
        let item = v
            .item
            .map(|v| Id::parse(&v, &mut self.interner, Some(namespace)).unwrap());

        self.registry.categories.insert(
            id,
            CategoryDef {
                id,
                ord,
                icon,
                icon_mode,
                item,
            },
        );

        Ok(())
    }

    pub fn load_categories(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let categories = dir.join("categories");

        for file in load_recursively(&categories, OsStr::new(RON_EXT)) {
            self.load_category(&file, namespace)?;
        }

        Ok(())
    }

    pub fn compile_categories(&mut self) {
        let mut ids = self.registry.categories.keys().cloned().collect::<Vec<_>>();

        ids.sort_by_key(|v| self.registry.categories[v].ord);

        let mut categories_tiles_map = HashMap::new();

        for tile in self.registry.tiles.values() {
            if let Some(category) = tile.category {
                categories_tiles_map
                    .entry(category)
                    .or_insert_with(Vec::new)
                    .push(tile.id)
            }
        }

        self.ordered_categories = ids;
        self.registry.categories_tiles_map = categories_tiles_map;
    }

    pub fn get_tiles_by_category(&self, id: Id) -> Option<&Vec<TileId>> {
        self.registry.categories_tiles_map.get(&id)
    }

    pub fn get_researches_by_category(&self, id: Id) -> Option<Vec<Id>> {
        self.registry.categories_tiles_map.get(&id).map(|tiles| {
            tiles
                .iter()
                .flat_map(|tile| self.get_research_by_unlock(*tile).map(|v| v.id))
                .collect()
        })
    }
}
