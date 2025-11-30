use core::{fmt::Display, time::Duration};
use std::{collections::BTreeMap, time::Instant};

use automancy_data::{
    id::ModelId,
    id_map::IdMap,
    math::{Matrix4, Quat, Transform, Vec3, Vec4},
};
use automancy_game::resources::{ResourceManager, global::debug_id};
use bytemuck::{Pod, Zeroable};
use gltf::animation::util::ReadOutputs;

use crate::{AnimPrimitive, AnimationChannel, AnimationFrame, Interpolation, gpu};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
struct GltfNodeId(u16);

impl From<u16> for GltfNodeId {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct MeshId(u32);

impl From<u32> for MeshId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Display for MeshId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("MeshId({})", self.0))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IndexRange {
    pub pos: u32,
    pub count: u32,
    pub base_vertex: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeshMetadata {
    pub opaque: bool,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub id: MeshId,
    pub model_id: ModelId,

    pub vertices: Vec<gpu::data::GpuGameVertex>,
    pub indices: Vec<u32>,

    pub transform: Matrix4,
    pub transform_inverted: Matrix4,

    pub metadata: MeshMetadata,
}

#[derive(Debug, Default)]
pub struct ModelManager {
    pub vertices: Vec<gpu::data::GpuGameVertex>,
    pub indices: Vec<u32>,

    model_id_map: IdMap<ModelId, Vec<MeshId>>,
    // TODO just use a vec maybe?
    pub meshes: BTreeMap<MeshId, Mesh>,
    pub index_ranges: BTreeMap<MeshId, IndexRange>,

    animation_channels_map: BTreeMap<MeshId, Vec<AnimationChannel<Transform>>>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl ModelManager {
    #[inline]
    pub fn model_count(&self) -> usize {
        self.model_id_map.len()
    }

    #[inline]
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    #[inline]
    pub fn animation_count(&self) -> usize {
        self.animation_channels_map.len()
    }

    #[inline]
    pub fn get_animation(&self, mesh_id: &MeshId) -> &Vec<AnimationChannel<Transform>> {
        static DEFAULT: Vec<AnimationChannel<Transform>> = vec![];

        self.animation_channels_map.get(mesh_id).unwrap_or(&DEFAULT)
    }

    #[inline]
    pub fn sample_animation(&mut self, mesh_id: &MeshId, frame_start: Instant) -> Matrix4 {
        if let Some(v) = self.animation_channels_map.get_mut(mesh_id) {
            let mut transform = Transform::default();
            for channel in v {
                transform = transform.add(channel.sample_at_instant(frame_start));
            }

            Matrix4::from(transform)
        } else {
            Matrix4::identity()
        }
    }

    #[inline]
    pub fn get_model_mesh_ids(&self, model_id: &ModelId) -> &[MeshId] {
        let Some(ids) = self.model_id_map.get(model_id) else {
            panic!("ModelId({}) should contain meshes", debug_id(*model_id))
        };

        ids
    }

    #[allow(clippy::type_complexity)]
    fn parse_gltf_model(
        (document, buffers): &(gltf::Document, Vec<gltf::buffer::Data>),
        mesh_count: &mut u32,
    ) -> (BTreeMap<GltfNodeId, Mesh>, BTreeMap<GltfNodeId, Vec<AnimationChannel<Transform>>>) {
        let mut meshes = BTreeMap::default();
        let mut animations = BTreeMap::default();

        for scene in document.scenes() {
            for node in scene.nodes() {
                let node_id = GltfNodeId::from(node.index() as u16);

                let transform = node.transform();
                let transform = {
                    let (translation, rotation, scale) = transform.decomposed();
                    Transform {
                        position: Vec3::from_slice(&translation),
                        orientation: Quat::from_vec4(Vec4::from_slice(&rotation)),
                        scale: Vec3::from_slice(&scale),
                    }
                };

                if let Some(mesh) = node.mesh() {
                    let mut read_vertices = vec![];
                    let mut read_indices = vec![];

                    for primitive in mesh.primitives() {
                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                        if let Some((positions, (normals, colors))) =
                            reader.read_positions().zip(reader.read_normals().zip(reader.read_colors(0)))
                        {
                            for (pos, (normal, color)) in positions.zip(normals.zip(colors.into_rgba_f32())) {
                                read_vertices.push(gpu::data::GpuGameVertex {
                                    pos,
                                    normal,
                                    color,
                                })
                            }
                        }

                        if let Some(indices) = reader.read_indices() {
                            for index in indices.into_u32() {
                                read_indices.push(index)
                            }
                        }
                    }

                    meshes.insert(
                        node_id,
                        Mesh {
                            id: MeshId(*mesh_count),
                            model_id: ModelId::invalid(),

                            metadata: MeshMetadata {
                                opaque: read_vertices.iter().all(|v| v.color[3] >= 1.0),
                            },

                            transform: Matrix4::from(transform),
                            transform_inverted: Matrix4::from(Transform {
                                position: -transform.position,
                                orientation: -transform.orientation,
                                scale: Vec3::one() / transform.scale,
                            }),
                            vertices: read_vertices,
                            indices: read_indices,
                        },
                    );
                    *mesh_count += 1;
                }
            }
        }

        for animation in document.animations() {
            for channel in animation.channels() {
                let node = channel.target().node();
                let node_id = GltfNodeId::from(node.index() as u16);

                let interpolation = Interpolation::from(channel.sampler().interpolation());

                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some((inputs, outputs)) = reader.read_inputs().zip(reader.read_outputs()) {
                    let mut translations = vec![];
                    let mut scales = vec![];
                    let mut rotations = vec![];

                    match outputs {
                        ReadOutputs::Translations(mut outputs) => match interpolation {
                            Interpolation::Linear | Interpolation::Step => {
                                for input in inputs {
                                    let output = outputs.next().unwrap();

                                    translations.push(AnimationFrame {
                                        timestamp: Duration::from_secs_f32(input),
                                        value: Transform {
                                            position: Vec3::from_slice(&output),
                                            ..Default::default()
                                        },
                                        in_tangent: None,
                                        out_tangent: None,
                                    });
                                }
                            },
                            Interpolation::CubicSpline => {
                                for input in inputs {
                                    let [in_tangent, output, out_tangent] =
                                        [outputs.next().unwrap(), outputs.next().unwrap(), outputs.next().unwrap()];

                                    translations.push(AnimationFrame {
                                        timestamp: Duration::from_secs_f32(input),
                                        value: Transform {
                                            position: Vec3::from_slice(&output),
                                            ..Default::default()
                                        },
                                        in_tangent: Some(Transform {
                                            position: Vec3::from_slice(&in_tangent),
                                            ..Default::default()
                                        }),
                                        out_tangent: Some(Transform {
                                            position: Vec3::from_slice(&out_tangent),
                                            ..Default::default()
                                        }),
                                    });
                                }
                            },
                        },
                        ReadOutputs::Scales(mut outputs) => match interpolation {
                            Interpolation::Linear | Interpolation::Step => {
                                for input in inputs {
                                    let output = outputs.next().unwrap();

                                    scales.push(AnimationFrame {
                                        timestamp: Duration::from_secs_f32(input),
                                        value: Transform {
                                            scale: Vec3::from_slice(&output),
                                            ..Default::default()
                                        },
                                        in_tangent: None,
                                        out_tangent: None,
                                    });
                                }
                            },
                            Interpolation::CubicSpline => {
                                for input in inputs {
                                    let [in_tangent, output, out_tangent] =
                                        [outputs.next().unwrap(), outputs.next().unwrap(), outputs.next().unwrap()];

                                    scales.push(AnimationFrame {
                                        timestamp: Duration::from_secs_f32(input),
                                        value: Transform {
                                            scale: Vec3::from_slice(&output),
                                            ..Default::default()
                                        },
                                        in_tangent: Some(Transform {
                                            scale: Vec3::from_slice(&in_tangent),
                                            ..Default::default()
                                        }),
                                        out_tangent: Some(Transform {
                                            scale: Vec3::from_slice(&out_tangent),
                                            ..Default::default()
                                        }),
                                    });
                                }
                            },
                        },
                        ReadOutputs::Rotations(outputs) => {
                            let mut outputs = outputs.into_f32();
                            match interpolation {
                                Interpolation::Linear | Interpolation::Step => {
                                    for input in inputs {
                                        let output = outputs.next().unwrap();

                                        rotations.push(AnimationFrame {
                                            timestamp: Duration::from_secs_f32(input),
                                            value: Transform {
                                                orientation: Quat::from_vec4(Vec4::from_slice(&output)),
                                                ..Default::default()
                                            },
                                            in_tangent: None,
                                            out_tangent: None,
                                        });
                                    }
                                },
                                Interpolation::CubicSpline => {
                                    for input in inputs {
                                        let [in_tangent, output, out_tangent] =
                                            [outputs.next().unwrap(), outputs.next().unwrap(), outputs.next().unwrap()];

                                        rotations.push(AnimationFrame {
                                            timestamp: Duration::from_secs_f32(input),
                                            value: Transform {
                                                orientation: Quat::from_vec4(Vec4::from_slice(&output)),
                                                ..Default::default()
                                            },
                                            in_tangent: Some(Transform {
                                                orientation: Quat::from_vec4(Vec4::from_slice(&in_tangent)),
                                                ..Default::default()
                                            }),
                                            out_tangent: Some(Transform {
                                                orientation: Quat::from_vec4(Vec4::from_slice(&out_tangent)),
                                                ..Default::default()
                                            }),
                                        });
                                    }
                                },
                            }
                        },
                        _ => {},
                    }

                    for mut keyframes in [translations, scales, rotations] {
                        if keyframes.is_empty() {
                            continue;
                        }

                        if keyframes.len() == 1 {
                            log::warn!("Animation only has {} keyframes!", keyframes.len());
                            continue;
                        }

                        keyframes.sort_by_key(|v| v.timestamp);

                        animations
                            .entry(node_id)
                            .or_insert_with(Vec::default)
                            .push(AnimationChannel::new(keyframes, interpolation, true));
                    }
                }
            }
        }

        (meshes, animations)
    }

    pub fn load_models(&mut self, resource_man: &ResourceManager) {
        self.vertices.clear();
        self.indices.clear();

        self.model_id_map.clear();
        self.meshes.clear();
        self.index_ranges.clear();

        self.animation_channels_map.clear();

        let models = {
            let mut mesh_count = 0;

            resource_man
                .models
                .iter()
                .map(|(id, v)| (id, Self::parse_gltf_model(v, &mut mesh_count)))
                .collect::<Vec<_>>()
        };

        {
            let mut base_index_count = 0;
            let mut base_vertex_count = 0;

            for (model_id, (mut meshes, animations)) in models {
                for mesh in meshes.values_mut() {
                    self.model_id_map.entry(model_id).or_default().push(mesh.id);
                    mesh.model_id = model_id;

                    self.meshes.insert(mesh.id, mesh.clone());

                    let index_count = mesh.indices.len() as u32;

                    self.index_ranges.insert(
                        mesh.id,
                        IndexRange {
                            pos: base_index_count,
                            count: index_count,
                            base_vertex: base_vertex_count,
                        },
                    );

                    base_index_count += index_count;
                    base_vertex_count += mesh.vertices.len() as i32;

                    self.indices.extend_from_slice(&mesh.indices);
                    self.vertices.extend_from_slice(&mesh.vertices);
                }

                for (node_id, animation) in animations {
                    let mesh = &meshes[&node_id];

                    assert_eq!(self.animation_channels_map.insert(mesh.id, animation), None);
                }
            }
        }
    }
}
