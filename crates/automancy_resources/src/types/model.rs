use std::{ffi::OsStr, fs::read_to_string, mem, path::Path};

use automancy_defs::{
    id::{Id, ModelId},
    rendering::{Animation, Mesh, Vertex, load_gltf_model},
};
use gltf;
use hashbrown::HashMap;
use log;
use serde::Deserialize;

use crate::{RON_EXT, ResourceManager, load_recursively};

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
    pub fn model_or_missing_tile(&self, id: &ModelId) -> ModelId {
        if self.all_meshes_anims.contains_key(id) {
            *id
        } else {
            ModelId(self.registry.model_ids.tile_missing)
        }
    }

    pub fn model_or_missing_item(&self, id: &ModelId) -> ModelId {
        if self.all_meshes_anims.contains_key(id) {
            *id
        } else {
            ModelId(self.registry.model_ids.item_missing)
        }
    }

    pub fn model_or_puzzle_space(&self, id: &ModelId) -> ModelId {
        if self.all_meshes_anims.contains_key(id) {
            *id
        } else {
            ModelId(self.registry.model_ids.puzzle_space)
        }
    }

    pub fn item_model_or_missing(&self, id: &Id) -> ModelId {
        if let Some(def) = self.registry.items.get(id) {
            if self.all_meshes_anims.contains_key(&def.model) {
                return def.model;
            }
        }

        ModelId(self.registry.model_ids.item_missing)
    }

    pub fn mesh_or_missing_tile_mesh(
        &self,
        id: &ModelId,
    ) -> (ModelId, &(Vec<Option<Mesh>>, Vec<Animation>)) {
        self.all_meshes_anims
            .get(id)
            .map(|v| (*id, v))
            .unwrap_or_else(|| {
                (
                    ModelId(self.registry.model_ids.tile_missing),
                    self.all_meshes_anims
                        .get(&ModelId(self.registry.model_ids.tile_missing))
                        .expect("'missing tile' model is missing from namespace core"),
                )
            })
    }

    fn load_model(&mut self, file: &Path, namespace: &str) -> anyhow::Result<()> {
        log::info!("Loading model at: {file:?}");

        let v = ron::from_str::<Raw>(&read_to_string(file)?)?;

        let file = file.parent().unwrap().join("files").join(v.file.as_str());

        log::info!("Loading model file at: {file:?}");

        let (document, buffers, _images) = gltf::import(file)?;

        let id = Id::parse(&v.id, &mut self.interner, Some(namespace)).unwrap();

        self.all_meshes_anims
            .insert(ModelId(id), load_gltf_model(document, buffers));

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
        self.all_meshes_anims
            .iter_mut()
            .for_each(|(id, (model, _))| {
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
