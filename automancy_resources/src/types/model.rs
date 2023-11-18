use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;

use serde::{Deserialize, Serialize};

use automancy_defs::gltf::animation::util::ReadOutputs;
use automancy_defs::gltf::json;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::{Id, IdRaw};
use automancy_defs::math::{Quaternion, Vector3};
use automancy_defs::rendering::{Animation, AnimationUnit, Model, Vertex};
use automancy_defs::{gltf, id, log};

use crate::data::item::Item;
use crate::{load_recursively, ResourceManager, RON_EXT};

#[derive(Debug, Default, Clone, Copy)]
pub struct IndexRange {
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelRaw {
    pub id: IdRaw,
    pub file: String,
}

impl ResourceManager {
    pub fn get_model(&self, model: Id) -> Id {
        if self.all_index_ranges.contains_key(&model) {
            model
        } else {
            self.registry.model_ids.missing
        }
    }

    pub fn get_item_model(&self, item: Item) -> Id {
        if self.all_index_ranges.contains_key(&item.model) {
            item.model
        } else {
            self.registry.model_ids.items_missing
        }
    }

    fn load_model(&mut self, file: &Path) -> anyhow::Result<()> {
        log::info!("Loading model at: {file:?}");

        let model: ModelRaw = ron::from_str(&read_to_string(file)?)?;

        let file = file
            .parent()
            .unwrap()
            .join("files")
            .join(model.file.as_str());

        log::info!("Loading model file at: {file:?}");

        let (document, buffers, _images) = gltf::import(file)?;

        let mut models = HashMap::new();
        let mut animations = vec![];

        for animation in document.animations() {
            for channel in animation.channels() {
                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

                let target = channel.target().node().index();
                let sampler = channel.sampler();
                let interpolation = sampler.interpolation();
                let mut read_inputs = vec![];
                let mut read_outputs = vec![];

                if let Some((inputs, outputs)) = reader.read_inputs().zip(reader.read_outputs()) {
                    let max = sampler
                        .output()
                        .max()
                        .as_ref()
                        .and_then(json::Value::as_f64)
                        .unwrap_or(0.0) as f32;
                    let min = sampler
                        .output()
                        .min()
                        .as_ref()
                        .and_then(json::Value::as_f64)
                        .unwrap_or(0.0) as f32;

                    match outputs {
                        ReadOutputs::Translations(outputs) => {
                            for (input, output) in inputs.zip(outputs) {
                                read_inputs.push(input);
                                read_outputs
                                    .push(AnimationUnit::Translation(Vector3::from(output)));
                            }
                        }
                        ReadOutputs::Scales(outputs) => {
                            for (input, output) in inputs.zip(outputs) {
                                read_inputs.push(input);
                                read_outputs.push(AnimationUnit::Scale(Vector3::from(output)));
                            }
                        }
                        ReadOutputs::Rotations(outputs) => {
                            for (input, output) in inputs.zip(outputs.into_f32()) {
                                read_inputs.push(input);
                                read_outputs.push(AnimationUnit::Rotate(Quaternion::from(output)));
                            }
                        }
                        _ => {}
                    }

                    animations.push(Animation {
                        target,
                        min,
                        max,
                        interpolation,
                        inputs: read_inputs,
                        outputs: read_outputs,
                    })
                }
            }
        }

        for scene in document.scenes() {
            for node in scene.nodes() {
                if let Some(mesh) = node.mesh() {
                    let name = mesh.name().unwrap_or("").to_string();

                    let index = node.index();

                    let mut read_vertices = vec![];
                    let mut read_indices = vec![];

                    for primitive in mesh.primitives() {
                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                        if let Some((positions, (normals, colors))) = reader
                            .read_positions()
                            .zip(reader.read_normals().zip(reader.read_colors(0)))
                        {
                            for (pos, (normal, color)) in
                                positions.zip(normals.zip(colors.into_rgba_f32()))
                            {
                                read_vertices.push(Vertex { pos, normal, color })
                            }
                        }

                        if let Some(indices) = reader.read_indices() {
                            for index in indices.into_u32() {
                                read_indices.push(index as u16)
                            }
                        }
                    }

                    models.insert(
                        mesh.index(),
                        Model {
                            vertices: read_vertices,
                            indices: read_indices,
                            name,
                            index,
                        },
                    );
                }
            }
        }

        self.all_models
            .insert(model.id.to_id(&mut self.interner), (models, animations));

        Ok(())
    }

    pub fn load_models(&mut self, dir: &Path) -> anyhow::Result<()> {
        let models = dir.join("models");

        for file in load_recursively(&models, OsStr::new(RON_EXT)) {
            self.load_model(&file)?;
        }

        Ok(())
    }

    pub fn compile_models(&mut self) -> (Vec<Vertex>, Vec<u16>) {
        let mut ids = self
            .registry
            .tiles
            .iter()
            .flat_map(|(id, _)| self.interner.resolve(*id))
            .map(IdRaw::parse)
            .collect::<Vec<_>>();

        ids.sort_unstable();

        if let Some(none_idx) =
            ids.iter().enumerate().find_map(
                |(idx, id)| {
                    if id == &id::NONE {
                        Some(idx)
                    } else {
                        None
                    }
                },
            )
        {
            ids.swap(none_idx, 0);
        }

        let ids = ids
            .into_iter()
            .flat_map(|id| self.interner.get(id.to_string()))
            .collect();

        self.ordered_tiles = ids;

        // indices vertices
        let mut vertices = vec![];
        let mut indices = HashMap::new();

        let mut index_offset_counter = 0;

        self.all_models.iter().for_each(|(id, (model, _))| {
            model.iter().for_each(|(index, model)| {
                vertices.push(model.vertices.clone());

                indices.entry(*id).or_insert_with(Vec::new).push((
                    *index,
                    model
                        .indices
                        .iter()
                        .map(|v| *v + index_offset_counter)
                        .collect::<Vec<_>>(),
                ));

                index_offset_counter += model.vertices.len() as u16;
            });
        });

        let vertices = vertices.into_iter().flatten().collect::<Vec<_>>();

        let mut offset_count = 0;

        let all_index_ranges = indices.iter().map(|(id, indices)| {
            let ranges = indices
                .iter()
                .map(|(index, v)| {
                    let size = v.len() as u32;

                    let range = IndexRange {
                        offset: offset_count,
                        size,
                    };

                    offset_count += size;

                    (*index, range)
                })
                .collect::<HashMap<_, _>>();

            (*id, ranges)
        });

        self.all_index_ranges = HashMap::from_iter(all_index_ranges);

        let indices = indices
            .into_iter()
            .flat_map(|(_id, indices)| indices.into_iter().flat_map(move |v| v.1))
            .collect();

        (vertices, indices)
    }
}
