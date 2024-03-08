use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::{Deserialize, Serialize};

use automancy_defs::id::{Id, IdRaw};
use automancy_defs::log;

use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CategoryRaw {
    pub id: IdRaw,
    pub ord: i64,
    pub icon: IdRaw,
    pub item: Option<IdRaw>,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub id: Id,
    pub ord: i64,
    pub icon: Id,
    pub item: Option<Id>,
}

impl ResourceManager {
    fn load_category(&mut self, file: &Path) -> anyhow::Result<()> {
        log::info!("Loading tag at: {file:?}");

        let category: CategoryRaw = ron::from_str(&read_to_string(file)?)?;

        let id = category.id.to_id(&mut self.interner);
        let ord = category.ord;
        let icon = category.icon.to_id(&mut self.interner);
        let item = category.item.map(|v| v.to_id(&mut self.interner));

        let tag = Category {
            id,
            ord,
            icon,
            item,
        };

        self.registry.categories.insert(id, tag);

        Ok(())
    }

    pub fn load_categories(&mut self, dir: &Path) -> anyhow::Result<()> {
        let categories = dir.join("categories");

        for file in load_recursively(&categories, OsStr::new(RON_EXT)) {
            self.load_category(&file)?;
        }

        Ok(())
    }

    pub fn ordered_categories(&mut self) {
        let mut ids = self.registry.categories.keys().cloned().collect::<Vec<_>>();

        ids.sort_by_key(|v| self.registry.categories[v].ord);

        self.ordered_categories = ids;
    }
}
