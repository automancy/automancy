use std::ffi::OsStr;
use std::path::Path;
use std::{fs::read_to_string, mem};

use hashbrown::HashMap;
use serde::Deserialize;

use automancy_defs::id::Id;
use automancy_defs::rendering::load_gltf_model;
use automancy_defs::rendering::Vertex;
use automancy_defs::{gltf, log};

use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Debug, Default, Clone, Copy)]
pub struct IndexRange {
    pub pos: u32,
    pub count: u32,
    pub base_vertex: i32,
}

#[derive(Debug, Deserialize)]
struct Raw {
    pub id: String,
    pub file: String,
}

impl ResourceManager {
    pub fn tile_model_or_missing(&self, id: Id) -> Id {
        if let Some(def) = self.registry.tiles.get(&id) {
            if self.all_models.contains_key(&def.model) {
                return def.model;
            }
        }

        self.registry.model_ids.missing
    }

    pub fn item_model_or_missing(&self, id: Id) -> Id {
        if let Some(def) = self.registry.items.get(&id) {
            if self.all_models.contains_key(&def.model) {
                return def.model;
            }
        }

        self.registry.model_ids.items_missing
    }

    fn load_model(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading model at: {file:?}");

        let v = ron::from_str::<Raw>(&read_to_string(file)?)?;

        let file = file.parent().unwrap().join("files").join(v.file.as_str());

        log::info!("Loading model file at: {file:?}");

        let (document, buffers, _images) = gltf::import(file)?;

        self.all_models.insert(
            Id::parse(&v.id, &mut self.interner, Some(namespace)).unwrap(),
            load_gltf_model(document, buffers),
        );

        Ok(())
    }

    pub fn load_models(&mut self, dir: &Path, namespace: &str) -> anyhow::Result<()> {
        let models = dir.join("models");

        for file in load_recursively(&models, OsStr::new(RON_EXT)) {
            self.load_model(&file, namespace)?;
        }

        Ok(())
    }

    pub fn compile_models(&mut self) -> (Vec<Vertex>, Vec<u16>) {
        let mut vertices = vec![];
        let mut indices = HashMap::new();

        let mut base_vertex_count = 0;
        self.all_models.iter_mut().for_each(|(id, (model, _))| {
            model.iter_mut().flatten().for_each(|mesh| {
                indices.entry(*id).or_insert_with(Vec::new).push((
                    mesh.index,
                    mem::take(&mut mesh.indices),
                    base_vertex_count,
                ));

                base_vertex_count += mesh.vertices.len() as i32;

                vertices.append(&mut mesh.vertices);
            });
        });

        let mut offset_count = 0;

        self.all_index_ranges = indices
            .iter()
            .map(|(id, indices)| {
                let ranges = indices
                    .iter()
                    .map(|(index, v, base_vertex)| {
                        let size = v.len() as u32;

                        let range = IndexRange {
                            pos: offset_count,
                            count: size,
                            base_vertex: *base_vertex,
                        };

                        offset_count += size;

                        (*index, range)
                    })
                    .collect::<HashMap<_, _>>();

                (*id, ranges)
            })
            .collect::<HashMap<_, _>>();

        let indices = indices
            .into_iter()
            .flat_map(|(_, indices)| indices.into_iter().flat_map(move |v| v.1))
            .collect();

        (vertices, indices)
    }
}
