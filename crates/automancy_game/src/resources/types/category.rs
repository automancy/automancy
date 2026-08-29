use std::{fs::read_to_string, path::Path};

use automancy_data::{
    id::{CategoryId, IdInterner, ItemId, ResearchId, TileId, deserialize::StrId},
    id_map::IdMap,
    math::Int,
    rendering::{GenericModel, deserialize::StrGenericModel},
};
use serde::Deserialize;

use crate::{
    persistent,
    resources::{MutableResourceManager, RON_EXTS, ResourceError, ResourceManager, read_recursively},
};

#[derive(Debug, Clone, Copy)]
pub struct CategoryDef {
    pub id: CategoryId,
    pub ord: Int,
    pub icon: GenericModel,
    pub item: ItemId,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: StrId,
    pub ord: Int,
    pub icon: StrGenericModel,
    pub item: Option<StrId>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_category_file(&mut self, interner: &mut IdInterner, path: &Path, namespace: &str) -> Result<(), ResourceError> {
        log::info!("Loading category at: {}.", path.display());

        let v = persistent::ron::ron_options().from_str::<Raw>(&read_to_string(path)?)?;

        let id = CategoryId(interner.get_or_intern(&v.id, Some(namespace))?);
        if id.is_built_in() {
            return Err(ResourceError::BuiltInRedefined {
                ty: "category",
                file: path.to_path_buf(),
                name: interner.resolve(*id).unwrap().to_string(),
            });
        }
        let ord = v.ord;
        let icon = v.icon.into_icon(interner, Some(namespace))?;
        let item = ItemId(interner.get_or_intern_opt(v.item.as_deref(), Some(namespace))?);

        self.registry.category_defs.insert(
            id,
            CategoryDef {
                id,
                ord,
                icon,
                item,
            },
        );

        Ok(())
    }

    pub fn load_category_files(dir: &Path, namespace: &str) {
        MutableResourceManager::with_interner(|resource_man, interner| {
            let path = dir.join("categories");

            for entry in read_recursively(&path, RON_EXTS) {
                match resource_man.load_category_file(interner, entry.path(), namespace) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
    }

    pub fn compile_categories(&self) -> (Vec<CategoryId>, IdMap<CategoryId, Vec<TileId>>) {
        let mut ids = self.registry.category_defs.keys().cloned().collect::<Vec<_>>();
        ids.sort_by_key(|v| self.registry.category_defs[v].ord);

        let mut category_tiles_map = IdMap::new();

        for tile in self.registry.tile_defs.values() {
            if !tile.category.is_none() {
                category_tiles_map.entry(tile.category).or_insert_with(Vec::new).push(tile.id)
            }
        }

        (ids, category_tiles_map)
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl ResourceManager {
    pub fn get_tiles_by_category(&self, id: CategoryId) -> Option<&Vec<TileId>> {
        self.registry.category_tiles_map.get(&id)
    }

    pub fn get_researches_by_category(&self, id: CategoryId) -> Option<Vec<ResearchId>> {
        self.registry.category_tiles_map.get(&id).map(|tiles| {
            tiles
                .iter()
                .flat_map(|tile| self.get_research_by_unlock(*tile).map(|v| v.id))
                .collect()
        })
    }
}
