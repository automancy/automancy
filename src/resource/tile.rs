use crate::resource::{LoadResource, ResourceManager, Tile, TileRaw, JSON_EXT};
use predicates::function::FnPredicate;
use predicates::prelude::predicate;
use predicates::Predicate;
use std::any::Any;
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

impl LoadResource<Tile> for ResourceManager {
    fn load(resource_man: &mut ResourceManager, file: &Path) -> Option<()> {
        log::info!("loading tile at {:?}", file);

        let tile: TileRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|_| panic!("error loading {file:?}")),
        )
        .unwrap_or_else(|_| panic!("error loading {file:?}"));

        let id = tile.id.to_id(&mut self.interner);

        if let Some(models) = tile.models {
            for model in models {
                let references = self
                    .models_referenced
                    .entry(model.to_id(&mut self.interner))
                    .or_insert_with(Vec::default);
                references.push(id);
            }
        }

        let scripts = tile.scripts.map(|v| {
            v.into_iter()
                .map(|id| id.to_id(&mut self.interner))
                .collect()
        });

        let tile_type = tile.tile_type;

        self.tiles.insert(
            id,
            Tile {
                tile_type,
                scripts,
                faces_indices: vec![],
            },
        );

        Some(())
    }

    const FILTER: Box<dyn Predicate<PathBuf>> = Box::new(predicate::function(|v: PathBuf| {
        v.extension() == Some(OsStr::new(JSON_EXT))
    }));

    const DIR: String = String::from("tiles");
}
