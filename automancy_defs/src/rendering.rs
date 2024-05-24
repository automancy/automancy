use std::{f32::consts::PI, mem::size_of};

use bytemuck::{Pod, Zeroable};

use glam::{vec3, vec4};
use gltf::{
    animation::{util::ReadOutputs, Interpolation},
    Document,
};
use gltf::{buffer::Data, scene::Transform};
use wgpu::{vertex_attr_array, BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::math::{direction_to_angle, Float, Matrix3, Matrix4, Quaternion, Vec2, Vec3, Vec4};

pub const LINE_DEPTH: Float = 0.1;

/// Produces a line shape.
pub fn make_line(a: Vec2, b: Vec2) -> Matrix4 {
    let mid = a.lerp(b, 0.5);
    let d = a.distance(b);
    let theta = direction_to_angle(b - a);

    Matrix4::from_translation(vec3(mid.x, mid.y, 0.1))
        * Matrix4::from_rotation_z(theta)
        * Matrix4::from_scale(vec3(d.max(0.001), 0.1, LINE_DEPTH))
}

// vertex

pub type VertexPos = [Float; 3];
pub type VertexColor = [Float; 4];
pub type RawMat4 = [[Float; 4]; 4];
pub type RawMat3 = [[Float; 3]; 3];

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialOrd, PartialEq, Zeroable, Pod)]
pub struct Vertex {
    pub pos: VertexPos,
    pub normal: VertexPos,
    pub color: VertexColor,
}

impl Vertex {
    pub fn desc() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x4,
        ];

        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

// instance

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceData {
    color_offset: VertexColor,
    alpha: Float,
    light_pos: Vec4,
    model_matrix: Matrix4,
    world_matrix: Option<Matrix4>,
}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            color_offset: Default::default(),
            alpha: 1.0,
            light_pos: vec4(0.0, 0.0, 1.0, 1.0),
            model_matrix: Matrix4::IDENTITY,
            world_matrix: None,
        }
    }
}

impl InstanceData {
    #[inline]
    pub fn add_model_matrix(mut self, matrix: Matrix4) -> Self {
        self.model_matrix *= matrix;

        self
    }

    #[inline]
    pub fn add_translation(mut self, translation: Vec3) -> Self {
        self.model_matrix *= Matrix4::from_translation(translation);

        self
    }

    #[inline]
    pub fn add_scale(mut self, scale: Vec3) -> Self {
        self.model_matrix *= Matrix4::from_scale(scale);

        self
    }

    #[inline]
    pub fn with_model_matrix(mut self, matrix: Matrix4) -> Self {
        self.model_matrix = matrix;

        self
    }

    #[inline]
    pub fn get_model_matrix(self) -> Matrix4 {
        self.model_matrix
    }

    #[inline]
    pub fn add_alpha(mut self, alpha: Float) -> Self {
        self.alpha *= alpha;

        self
    }

    #[inline]
    pub fn with_alpha(mut self, alpha: Float) -> Self {
        self.alpha = alpha;

        self
    }

    #[inline]
    pub fn with_light_pos(mut self, light_pos: Vec3, light_strength: Option<Float>) -> Self {
        self.light_pos = light_pos.extend(light_strength.unwrap_or(1.0));

        self
    }

    #[inline]
    pub fn with_color_offset(mut self, color_offset: impl Into<VertexColor>) -> Self {
        self.color_offset = color_offset.into();

        self
    }

    #[inline]
    pub fn with_world_matrix(mut self, matrix: Matrix4) -> Self {
        self.world_matrix = Some(matrix);

        self
    }

    #[inline]
    pub fn get_world_matrix(self) -> Option<Matrix4> {
        self.world_matrix
    }

    #[inline]
    pub fn add_world_matrix_right(mut self, matrix: Matrix4) -> Self {
        if let Some(s) = self.world_matrix {
            self.world_matrix = Some(s * matrix);
        } else {
            self.world_matrix = Some(matrix);
        }

        self
    }

    #[inline]
    pub fn add_world_matrix_left(mut self, matrix: Matrix4) -> Self {
        if let Some(s) = self.world_matrix {
            self.world_matrix = Some(matrix * s);
        } else {
            self.world_matrix = Some(matrix);
        }

        self
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialOrd, PartialEq, Zeroable, Pod)]
pub struct MatrixData {
    model_matrix: RawMat4,
    world_matrix: RawMat4,
    normal_matrix: [[Float; 4]; 3], // memory alignment issue, padded to 16 bytes
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialOrd, PartialEq, Zeroable, Pod)]
pub struct RawInstanceData {
    color_offset: VertexColor,
    alpha: Float,
    light_pos: [Float; 4],
    matrix_index: u32,
}

static FIX_COORD: Matrix4 = Matrix4::from_cols(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, -1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.0, 0.0, 0.0, 1.0),
);

impl RawInstanceData {
    pub fn from_instance(instance: InstanceData, buffer: &mut Vec<MatrixData>) -> Self {
        let world_matrix = if let Some(world_matrix) = instance.world_matrix {
            FIX_COORD * world_matrix
        } else {
            FIX_COORD
        };
        let model_matrix = instance.model_matrix;

        if buffer.last().map(|v| (v.world_matrix, v.model_matrix))
            != Some((
                world_matrix.to_cols_array_2d(),
                model_matrix.to_cols_array_2d(),
            ))
        {
            let inverse_transpose = Matrix3::from_cols(
                model_matrix.x_axis.truncate(),
                model_matrix.y_axis.truncate(),
                model_matrix.z_axis.truncate(),
            )
            .inverse()
            .transpose();

            let matrix_data = MatrixData {
                model_matrix: model_matrix.to_cols_array_2d(),
                world_matrix: world_matrix.to_cols_array_2d(),
                normal_matrix: [
                    inverse_transpose.x_axis.extend(0.0).to_array(),
                    inverse_transpose.y_axis.extend(0.0).to_array(),
                    inverse_transpose.z_axis.extend(0.0).to_array(),
                ],
            };

            buffer.push(matrix_data);
        }
        let index = buffer.len() - 1;

        Self {
            color_offset: instance.color_offset,
            alpha: instance.alpha,
            light_pos: [
                instance.light_pos.x,
                instance.light_pos.y,
                instance.light_pos.z,
                instance.light_pos.w,
            ],
            matrix_index: index as u32,
        }
    }

    pub fn desc() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array![
            3 => Float32x4,
            4 => Float32,
            5 => Float32x4,
            6 => Uint32,
        ];

        VertexBufferLayout {
            array_stride: size_of::<RawInstanceData>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: ATTRIBUTES,
        }
    }
}

// UBO

pub static DEFAULT_LIGHT_COLOR: VertexColor = [1.0; 4];

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct GameUBO {
    light_color: VertexColor,
    camera_matrix: RawMat4,
}

impl GameUBO {
    pub fn new(camera_matrix: Matrix4) -> Self {
        Self {
            light_color: DEFAULT_LIGHT_COLOR,
            camera_matrix: camera_matrix.to_cols_array_2d(),
        }
    }
}

impl Default for GameUBO {
    fn default() -> Self {
        Self::new(Matrix4::IDENTITY)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct PostProcessingUBO {
    pub camera_matrix: RawMat4,
}

impl Default for PostProcessingUBO {
    fn default() -> Self {
        Self {
            camera_matrix: Matrix4::IDENTITY.to_cols_array_2d(),
        }
    }
}

// model

#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub target: usize,
    pub interpolation: Interpolation,
    pub inputs: Vec<Float>,
    pub outputs: Vec<Matrix4>,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub name: String,
    pub index: usize,
    pub matrix: Matrix4,
    pub transform: Transform,
}

pub fn load_gltf_model(
    document: Document,
    buffers: Vec<Data>,
) -> (Vec<Option<Mesh>>, Vec<Animation>) {
    let mut meshes = Vec::new();
    let mut animations = vec![];

    for scene in document.scenes() {
        for node in scene.nodes() {
            let index = node.index();

            if let Some(mesh) = node.mesh() {
                let name = mesh.name().unwrap_or("").to_string();

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

                let transform = node.transform();

                meshes.resize(mesh.index() + 1, None);
                meshes[mesh.index()] = Some(Mesh {
                    vertices: read_vertices,
                    indices: read_indices,
                    name,
                    index,
                    matrix: Matrix4::from_rotation_z(PI)
                        * Matrix4::from_cols_array_2d(&transform.clone().matrix()),
                    transform,
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
                match outputs {
                    ReadOutputs::Translations(outputs) => {
                        let transform = meshes[target]
                            .as_ref()
                            .unwrap()
                            .transform
                            .clone()
                            .decomposed();
                        let [ox, oy, oz] = transform.0;
                        let [sx, sy, sz] = transform.2;

                        for (input, [x, y, z]) in inputs.zip(outputs) {
                            read_inputs.push(input);
                            read_outputs.push(Matrix4::from_translation(vec3(
                                (ox - x) / sx,
                                (oy - y) / sy,
                                (oz - z) / sz,
                            )));
                        }
                    }
                    ReadOutputs::Scales(outputs) => {
                        let [sx, sy, sz] = meshes[target]
                            .as_ref()
                            .unwrap()
                            .transform
                            .clone()
                            .decomposed()
                            .2;

                        for (input, [x, y, z]) in inputs.zip(outputs) {
                            read_inputs.push(input);
                            read_outputs.push(Matrix4::from_scale(vec3(x / sx, y / sy, z / sz)));
                        }
                    }
                    ReadOutputs::Rotations(outputs) => {
                        for (input, output) in inputs.zip(outputs.into_f32()) {
                            let transform = meshes[target]
                                .as_ref()
                                .unwrap()
                                .transform
                                .clone()
                                .decomposed();
                            let rotate = Quaternion::from_array(transform.1);
                            let output = Quaternion::from_array(output);

                            read_inputs.push(input);
                            read_outputs.push(Matrix4::from_quat(rotate.inverse() * output));
                        }
                    }
                    _ => {}
                }

                animations.push(Animation {
                    target,
                    interpolation,
                    inputs: read_inputs,
                    outputs: read_outputs,
                })
            }
        }
    }

    (meshes, animations)
}
