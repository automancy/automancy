use core::mem;

use automancy_data::{
    id::ModelId,
    math::{Matrix4, Quat, Vec3, Vec4},
};
use automancy_game::resources::{ResourceManager, types::model::IndexRange};
use gltf::animation::util::ReadOutputs;
use hashbrown::HashMap;

use crate::{
    data::GpuVertex,
    model::{Animation, GlobalMeshId, Mesh, MeshId},
};

pub mod data;
//pub mod game;
//pub mod gpu;
pub mod instance;
pub mod model;
//pub mod ui;

#[derive(Debug, Default)]
pub struct AutomancyRenderer {
    pub global_id_map: HashMap<(ModelId, MeshId), GlobalMeshId>,
    pub model_meshes: HashMap<ModelId, Vec<MeshId>>,
    pub all_index_ranges: HashMap<GlobalMeshId, IndexRange>,
}

impl AutomancyRenderer {
    fn parse_gltf_model(
        (document, buffers): (gltf::Document, Vec<gltf::buffer::Data>),
    ) -> (Vec<Option<Mesh>>, Vec<Animation>) {
        let mut meshes = Vec::new();
        let mut animations = vec![];

        for scene in document.scenes() {
            for node in scene.nodes() {
                let index = node.index();

                let transform = node.transform();
                let matrix = // Matrix4::rotation_z(consts::PI) *
                Matrix4::from_col_arrays(transform.clone().matrix());

                if let Some(mesh) = node.mesh() {
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
                                let pos = matrix.mul_point(Vec3::from_slice(&pos)).into_array();

                                read_vertices.push(GpuVertex { pos, normal, color })
                            }
                        }

                        if let Some(indices) = reader.read_indices() {
                            for index in indices.into_u32() {
                                read_indices.push(index as u16)
                            }
                        }
                    }

                    meshes.resize(mesh.index() + 1, None);
                    meshes[mesh.index()] = Some(Mesh {
                        index: MeshId::from(index as u16),

                        opaque: read_vertices.iter().all(|v| v.color[3] >= 1.0),
                        transform,

                        vertices: read_vertices,
                        indices: read_indices,
                    });
                }
            }
        }

        for animation in document.animations() {
            for channel in animation.channels() {
                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

                let target = channel.target().node().index();
                let sampler = channel.sampler();
                let interpolation = sampler.interpolation();
                let mut read_inputs = vec![];
                let mut read_outputs = vec![];

                if let Some((inputs, outputs)) = reader.read_inputs().zip(reader.read_outputs()) {
                    let (translation, rotation, scale) = meshes[target]
                        .as_ref()
                        .unwrap()
                        .transform
                        .clone()
                        .decomposed();

                    match outputs {
                        ReadOutputs::Translations(outputs) => {
                            let [ox, oy, oz] = translation;
                            let [sx, sy, sz] = scale;

                            for (input, [x, y, z]) in inputs.zip(outputs) {
                                read_inputs.push(input);
                                read_outputs.push(Matrix4::translation_3d(Vec3::new(
                                    (ox - x) / sx,
                                    (oy - y) / sy,
                                    (oz - z) / sz,
                                )));
                            }
                        }
                        ReadOutputs::Scales(outputs) => {
                            let [sx, sy, sz] = scale;

                            for (input, [x, y, z]) in inputs.zip(outputs) {
                                read_inputs.push(input);
                                read_outputs.push(Matrix4::scaling_3d(Vec3::new(
                                    x / sx,
                                    y / sy,
                                    z / sz,
                                )));
                            }
                        }
                        ReadOutputs::Rotations(outputs) => {
                            for (input, output) in inputs.zip(outputs.into_f32()) {
                                let rotate = Quat::from_vec4(Vec4::from_slice(&rotation));
                                let output = Quat::from_vec4(Vec4::from_slice(&output));

                                read_inputs.push(input);
                                read_outputs.push(Matrix4::from(rotate.inverse() * output));
                            }
                        }
                        _ => {}
                    }

                    animations.push(Animation {
                        target: MeshId::from(target as u16),
                        interpolation,
                        inputs: read_inputs,
                        outputs: read_outputs,
                    })
                }
            }
        }

        (meshes, animations)
    }

    pub fn compile_models(&mut self, resource_man: &ResourceManager) -> (Vec<GpuVertex>, Vec<u16>) {
        let mut vertices = Vec::new();
        let mut indices = HashMap::new();

        let mut models = resource_man
            .gltf_models
            .clone()
            .into_iter()
            .map(|(key, value)| (key, Self::parse_gltf_model(value)))
            .collect::<Vec<_>>();

        {
            let mut base_vertex_count = 0;

            models.iter_mut().for_each(|(model_id, (model, _))| {
                model.iter_mut().flatten().for_each(|mesh| {
                    indices.entry(*model_id).or_insert_with(Vec::new).push((
                        mesh.index,
                        mem::take(&mut mesh.indices),
                        base_vertex_count,
                    ));

                    base_vertex_count += mesh.vertices.len() as i32;

                    vertices.append(&mut mesh.vertices);
                });
            });
        }

        {
            let mut mesh_count = 0;
            let mut offset_count = 0;

            self.all_index_ranges = indices
                .iter()
                .flat_map(|(model_id, indices)| {
                    indices
                        .iter()
                        .map(|(mesh_id, v, base_vertex)| {
                            self.model_meshes
                                .entry(*model_id)
                                .or_default()
                                .push(*mesh_id);

                            let global_mesh_id = GlobalMeshId::from(mesh_count);
                            debug_assert_eq!(
                                self.global_id_map
                                    .insert((*model_id, *mesh_id), global_mesh_id),
                                None,
                            );
                            mesh_count += 1;

                            let size = v.len() as u32;
                            let range = IndexRange {
                                pos: offset_count,
                                count: size,
                                base_vertex: *base_vertex,
                            };
                            offset_count += size;

                            (global_mesh_id, range)
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<HashMap<_, _>>();
        }

        let indices = indices
            .into_iter()
            .flat_map(|(_, indices)| indices.into_iter().flat_map(move |v| v.1))
            .collect();

        (vertices, indices)
    }
}
