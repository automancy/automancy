use automancy_data::math::{Float, Matrix4, Quat, Vec3, Vec4};
use bytemuck::{Pod, Zeroable};
use gltf::animation::util::ReadOutputs;

use crate::data::GpuVertex;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct MeshId(u32);

#[derive(Debug, Clone)]
pub struct Mesh {
    pub index: MeshId,

    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u16>,

    pub opaque: bool,
    transform: gltf::scene::Transform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub target: MeshId,
    pub interpolation: gltf::animation::Interpolation,
    pub inputs: Vec<Float>,
    pub outputs: Vec<Matrix4>,
}

pub fn parse_gltf_model(
    document: gltf::Document,
    buffers: Vec<gltf::buffer::Data>,
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
                    index: MeshId(index as u32),

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
                    target: MeshId(target as u32),
                    interpolation,
                    inputs: read_inputs,
                    outputs: read_outputs,
                })
            }
        }
    }

    (meshes, animations)
}
