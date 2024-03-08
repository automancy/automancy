use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use egui::NumExt;
use egui_wgpu::wgpu::{
    vertex_attr_array, BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode,
};
use glam::{vec3, vec4};
use gltf::animation::Interpolation;
use gltf::scene::Transform;

use crate::math::{direction_to_angle, Float, Matrix3, Matrix4, Vec2, Vec3, Vec4};

pub const LINE_DEPTH: Float = 0.1;

/// Produces a line shape.
pub fn make_line(a: Vec2, b: Vec2) -> Matrix4 {
    let mid = a.lerp(b, 0.5);
    let d = a.distance(b);
    let theta = direction_to_angle(b - a);

    Matrix4::from_translation(vec3(mid.x, mid.y, 0.1))
        * Matrix4::from_rotation_z(theta)
        * Matrix4::from_scale(vec3(d.at_least(0.001), 0.1, LINE_DEPTH))
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

#[derive(Clone, Copy, Debug)]
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
            light_pos: vec4(0.0, 0.0, 0.0, 0.0),
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
    pub fn with_color_offset(mut self, color_offset: VertexColor) -> Self {
        self.color_offset = color_offset;

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
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
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
        let inverse_transpose = Matrix3::from_cols(
            model_matrix.x_axis.truncate(),
            model_matrix.y_axis.truncate(),
            model_matrix.z_axis.truncate(),
        )
        .inverse()
        .transpose();

        let matrix_data = MatrixData {
            model_matrix: model_matrix.to_cols_array_2d(),
            normal_matrix: [
                inverse_transpose.x_axis.extend(0.0).to_array(),
                inverse_transpose.y_axis.extend(0.0).to_array(),
                inverse_transpose.z_axis.extend(0.0).to_array(),
            ],
            world_matrix: world_matrix.to_cols_array_2d(),
        };

        buffer.push(matrix_data);
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
}

impl Default for GameUBO {
    fn default() -> Self {
        Self {
            light_color: DEFAULT_LIGHT_COLOR,
        }
    }
}

// model

#[derive(Debug, Clone)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub name: String,
    pub index: usize,
    pub matrix: Matrix4,
    pub transform: Transform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub target: usize,
    pub interpolation: Interpolation,
    pub inputs: Vec<Float>,
    pub outputs: Vec<Matrix4>,
}
