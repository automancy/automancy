use core::fmt::Display;

use automancy_data::{
    id::ModelId,
    math::{Float, Matrix4, Quat, Transform, Vec3, Vec4, consts},
};
use automancy_game::resources::ResourceManager;
use bytemuck::{Pod, Zeroable};
use gltf::animation::util::ReadOutputs;
use hashbrown::HashMap;

use crate::gpu::data::GpuVertex;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct MeshId(u16);

impl From<u16> for MeshId {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct GlobalMeshId(u32);

impl From<u32> for GlobalMeshId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Display for GlobalMeshId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("GlobalMeshId({})", self.0))
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

    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u16>,

    pub transform: Matrix4,
    pub inverted: Matrix4,
    pub metadata: MeshMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationOutput {
    Translation(Vec3),
    Rotation(Quat),
    Scale(Vec3),
}

impl AnimationOutput {
    pub fn apply_to_transform(self, mut tranform: Transform) -> Transform {
        match self {
            AnimationOutput::Translation(v) => tranform.position += v,
            AnimationOutput::Rotation(v) => tranform.orientation = tranform.orientation * v,
            AnimationOutput::Scale(v) => tranform.scale *= v,
        }

        tranform
    }
}

/// (copied from [`gltf::animation::Interpolation`])
/// Specifies an interpolation algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Interpolation {
    /// Linear interpolation.
    ///
    /// The animated values are linearly interpolated between keyframes.
    /// When targeting a rotation, spherical linear interpolation (slerp) should be
    /// used to interpolate quaternions. The number output of elements must equal
    /// the number of input elements.
    Linear,

    /// Step interpolation.
    ///
    /// The animated values remain constant to the output of the first keyframe,
    /// until the next keyframe. The number of output elements must equal the number
    /// of input elements.
    Step,

    /// Cubic spline interpolation.
    ///
    /// The animation's interpolation is computed using a cubic spline with specified
    /// tangents. The number of output elements must equal three times the number of
    /// input elements. For each input element, the output stores three elements, an
    /// in-tangent, a spline vertex, and an out-tangent. There must be at least two
    /// keyframes when using this interpolation
    CubicSpline,
}

impl From<gltf::animation::Interpolation> for Interpolation {
    fn from(value: gltf::animation::Interpolation) -> Self {
        match value {
            gltf::animation::Interpolation::Linear => Interpolation::Linear,
            gltf::animation::Interpolation::Step => Interpolation::Step,
            gltf::animation::Interpolation::CubicSpline => Interpolation::CubicSpline,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationChannel {
    pub interpolation: Interpolation,
    pub duration: Float,
    pub keyframes: Vec<(Float, Matrix4)>,
}

#[derive(Debug, Default)]
pub struct ModelManager {
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u16>,

    global_mesh_ids: HashMap<(ModelId, MeshId), GlobalMeshId>,

    model_mesh_ids: HashMap<ModelId, Vec<MeshId>>,
    global_model_mesh_ids: HashMap<ModelId, Vec<GlobalMeshId>>,

    pub mesh_metadata: HashMap<GlobalMeshId, MeshMetadata>,
    pub index_ranges: HashMap<GlobalMeshId, IndexRange>,
    pub animation_channels_map: HashMap<GlobalMeshId, AnimationChannel>,
}

impl ModelManager {
    pub fn model_mesh_ids(&self, key: ModelId, fallback: ModelId) -> &Vec<MeshId> {
        self.model_mesh_ids.get(&key).unwrap_or_else(|| &self.model_mesh_ids[&fallback])
    }

    pub fn global_model_mesh_ids(&self, key: ModelId, fallback: ModelId) -> &Vec<GlobalMeshId> {
        self.global_model_mesh_ids
            .get(&key)
            .unwrap_or_else(|| &self.global_model_mesh_ids[&fallback])
    }

    fn parse_gltf_model((document, buffers): (gltf::Document, Vec<gltf::buffer::Data>)) -> (Vec<Option<Mesh>>, HashMap<MeshId, AnimationChannel>) {
        let mut meshes = Vec::new();
        let mut animations = HashMap::new();

        for scene in document.scenes() {
            for node in scene.nodes() {
                let index = node.index();

                let transform = node.transform();
                let transform = Matrix4::rotation_z(consts::PI) * Matrix4::from_col_arrays(transform.clone().matrix());

                if let Some(mesh) = node.mesh() {
                    let mut read_vertices = vec![];
                    let mut read_indices = vec![];

                    for primitive in mesh.primitives() {
                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                        if let Some((positions, (normals, colors))) = reader.read_positions().zip(reader.read_normals().zip(reader.read_colors(0))) {
                            for (pos, (normal, color)) in positions.zip(normals.zip(colors.into_rgba_f32())) {
                                let pos = transform.mul_point(Vec3::from_slice(&pos)).into_array();

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
                        metadata: MeshMetadata {
                            opaque: read_vertices.iter().all(|v| v.color[3] >= 1.0),
                        },

                        id: MeshId::from(index as u16),

                        transform,
                        inverted: transform.inverted(),
                        vertices: read_vertices,
                        indices: read_indices,
                    });
                }
            }
        }

        for animation in document.animations() {
            for channel in animation.channels() {
                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

                let mesh = channel.target().node();

                let mesh_id = MeshId::from(mesh.index() as u16);
                let mesh = meshes[mesh.index()].as_ref().unwrap();

                let mut read_values = vec![];
                if let Some((inputs, outputs)) = reader.read_inputs().zip(reader.read_outputs()) {
                    match outputs {
                        ReadOutputs::Translations(outputs) => {
                            for (input, output) in inputs.zip(outputs) {
                                read_values.push((input, AnimationOutput::Translation(Vec3::from_slice(&output))));
                            }
                        }
                        ReadOutputs::Scales(outputs) => {
                            for (input, output) in inputs.zip(outputs) {
                                read_values.push((input, AnimationOutput::Scale(Vec3::from_slice(&output))));
                            }
                        }
                        ReadOutputs::Rotations(outputs) => {
                            for (input, output) in inputs.zip(outputs.into_f32()) {
                                read_values.push((input, AnimationOutput::Rotation(Quat::from_vec4(Vec4::from_slice(&output)))));
                            }
                        }
                        _ => {}
                    }

                    let mut transforms = Vec::new();
                    for (input, output) in read_values {
                        if let Some((_, transform)) = transforms.iter_mut().find(|(v, _)| *v == input) {
                            *transform = output.apply_to_transform(*transform);
                        } else {
                            transforms.push((input, output.apply_to_transform(Transform::default())));
                        }
                    }
                    transforms.sort_by(|(a, ..), (b, ..)| a.total_cmp(b));

                    let sampler = channel.sampler();
                    let interpolation = sampler.interpolation();

                    animations.insert(
                        mesh_id,
                        AnimationChannel {
                            interpolation: interpolation.into(),
                            duration: transforms.last().unwrap().0,
                            keyframes: transforms
                                .into_iter()
                                .map(|(k, v)| (k, (mesh.transform * Matrix4::from(v) * mesh.inverted)))
                                .collect(),
                        },
                    );
                }
            }
        }

        (meshes, animations)
    }

    pub fn load_models(&mut self, resource_man: &ResourceManager) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let models = resource_man
            .gltf_models
            .clone()
            .into_iter()
            .map(|(id, v)| (id, Self::parse_gltf_model(v)))
            .collect::<Vec<_>>();

        {
            let mut base_index_count = 0;
            let mut base_vertex_count = 0;
            let mut mesh_count = 0;

            for (model_id, (meshes, animations)) in models {
                for mut mesh in meshes.into_iter().flatten() {
                    self.model_mesh_ids.entry(model_id).or_default().push(mesh.id);

                    let index_count = mesh.indices.len() as u32;

                    {
                        let global_mesh_id = GlobalMeshId::from(mesh_count);
                        assert_eq!(self.global_mesh_ids.insert((model_id, mesh.id), global_mesh_id), None,);
                        mesh_count += 1;

                        self.mesh_metadata.insert(global_mesh_id, std::mem::take(&mut mesh.metadata));

                        self.index_ranges.insert(
                            global_mesh_id,
                            IndexRange {
                                pos: base_index_count,
                                count: index_count,
                                base_vertex: base_vertex_count,
                            },
                        );
                    }

                    base_index_count += index_count;
                    base_vertex_count += mesh.vertices.len() as i32;

                    indices.append(&mut mesh.indices);
                    vertices.append(&mut mesh.vertices);
                }

                for (mesh_id, animation) in animations {
                    let global_id = self.global_mesh_ids[&(model_id, mesh_id)];
                    assert_eq!(self.animation_channels_map.insert(global_id, animation), None);
                }
            }
        }

        for (&(model_id, _), &global_id) in &self.global_mesh_ids {
            self.global_model_mesh_ids.entry(model_id).or_default().push(global_id);
        }

        self.vertices = vertices;
        self.indices = indices;
    }
}
