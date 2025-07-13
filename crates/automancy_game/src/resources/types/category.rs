use std::{ffi::OsStr, fs::read_to_string, path::Path};

use automancy_data::{
    id::{
        Id, TileId,
        deserialize::{StrId, StrIdExt},
    },
    math::Int,
};
use hashbrown::HashMap;
use serde::Deserialize;

use crate::{
    persistent,
    resources::{RON_EXT, ResourceManager, load_recursively, types::IconMode},
};

#[derive(Debug, Clone, Copy)]
pub struct CategoryDef {
    pub id: Id,
    pub ord: Int,
    pub icon: Id,
    pub icon_mode: IconMode,
    pub item: Option<Id>,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: StrId,
    pub ord: Int,
    pub icon: StrId,
    pub icon_mode: IconMode,
    pub item: Option<StrId>,
}

impl ResourceManager {
    fn load_category(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading category at: {file:?}");

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(file)?)?;

        let id = v.id.into_id(&mut self.interner, Some(namespace))?;
        let ord = v.ord;
        let icon = v.icon.into_id(&mut self.interner, Some(namespace))?;
        let icon_mode = v.icon_mode;
        let item = v.item.into_id(&mut self.interner, Some(namespace))?;

        self.registry.categorie_defs.insert(
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
        let mut ids = self.registry.categorie_defs.keys().cloned().collect::<Vec<_>>();

        ids.sort_by_key(|v| self.registry.categorie_defs[v].ord);

        let mut categories_tiles_map = HashMap::new();

        for tile in self.registry.tile_defs.values() {
            if let Some(category) = tile.category {
                categories_tiles_map.entry(category).or_insert_with(Vec::new).push(tile.id)
            }
        }

        self.ordered_categories = ids;
        self.registry.categories_tiles_map = categories_tiles_map;
    }

    pub fn get_tiles_by_category(&self, id: Id) -> Option<&Vec<TileId>> {
        self.registry.categories_tiles_map.get(&id)
    }

    pub fn get_researches_by_category(&self, id: Id) -> Option<Vec<Id>> {
        self.registry
            .categories_tiles_map
            .get(&id)
            .map(|tiles| tiles.iter().flat_map(|tile| self.get_research_by_unlock(*tile).map(|v| v.id)).collect())
    }
}
