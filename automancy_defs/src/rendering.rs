use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use cgmath::SquareMatrix;
use egui::ecolor::{linear_f32_from_gamma_u8, linear_f32_from_linear_u8};
use ply_rs::ply::{Property, PropertyAccess};
use wgpu::{vertex_attr_array, BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::math::{Float, Matrix4, Point3, Vector3};

// vertex

pub type VertexPos = [Float; 3];
pub type VertexColor = [Float; 4];
pub type RawMat4 = [[Float; 4]; 4];

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable, Pod)]
pub struct Vertex {
    pub pos: VertexPos,
    pub color: VertexColor,
    pub normal: VertexPos,
}

impl Vertex {
    pub fn desc() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array![
            0 => Float32x3,
            1 => Float32x4,
            2 => Float32x3,
        ];

        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

impl PropertyAccess for Vertex {
    fn new() -> Self {
        Self {
            pos: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 0.0],
        }
    }

    fn set_property(&mut self, key: String, property: Property) {
        match (key.as_ref(), property) {
            ("x", Property::Float(v)) => self.pos[0] = v,
            ("y", Property::Float(v)) => self.pos[1] = v,
            ("z", Property::Float(v)) => self.pos[2] = v,
            ("red", Property::UChar(v)) => self.color[0] = linear_f32_from_gamma_u8(v),
            ("green", Property::UChar(v)) => self.color[1] = linear_f32_from_gamma_u8(v),
            ("blue", Property::UChar(v)) => self.color[2] = linear_f32_from_gamma_u8(v),
            ("alpha", Property::UChar(v)) => self.color[3] = linear_f32_from_linear_u8(v),
            ("nx", Property::Float(v)) => self.normal[0] = v,
            ("ny", Property::Float(v)) => self.normal[1] = v,
            ("nz", Property::Float(v)) => self.normal[2] = v,
            (_, _) => {}
        }
    }
}

// instance

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct RawInstanceData {
    pub color_offset: VertexColor,
    pub model_matrix: RawMat4,
}

impl RawInstanceData {
    pub fn desc() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array![
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4,
        ];

        VertexBufferLayout {
            array_stride: size_of::<RawInstanceData>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: ATTRIBUTES,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InstanceData {
    pub color_offset: VertexColor,
    pub model_matrix: Matrix4,
}

impl From<InstanceData> for RawInstanceData {
    fn from(value: InstanceData) -> Self {
        Self {
            color_offset: value.color_offset,
            model_matrix: value.model_matrix.into(),
        }
    }
}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            color_offset: [0.0, 0.0, 0.0, 0.0],
            model_matrix: Matrix4::identity(),
        }
    }
}

impl InstanceData {
    #[inline]
    pub fn add_model_matrix(mut self, model_matrix: Matrix4) -> Self {
        self.model_matrix = self.model_matrix * model_matrix;

        self
    }

    #[inline]
    pub fn add_translation(mut self, translation: Vector3) -> Self {
        self.model_matrix = self.model_matrix * Matrix4::from_translation(translation);

        self
    }

    #[inline]
    pub fn add_scale(mut self, scale: Float) -> Self {
        self.model_matrix = self.model_matrix * Matrix4::from_scale(scale);

        self
    }

    #[inline]
    pub fn with_model_matrix(mut self, model_matrix: Matrix4) -> Self {
        self.model_matrix = model_matrix;

        self
    }

    #[inline]
    pub fn with_color_offset(mut self, color_offset: VertexColor) -> Self {
        self.color_offset = color_offset;

        self
    }
}

// UBO

pub static DEFAULT_LIGHT_COLOR: VertexColor = [0.9, 0.9, 0.9, 0.9];

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct GameUBO {
    light_pos: [f32; 4],
    light_color: VertexColor,
    world: RawMat4,
}

static FIX_COORD: RawMat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, -1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

impl Default for GameUBO {
    fn default() -> Self {
        Self {
            light_pos: [0.0, 0.0, 8.0, 1.0],
            light_color: DEFAULT_LIGHT_COLOR,
            world: FIX_COORD,
        }
    }
}

impl GameUBO {
    pub fn new(world: Matrix4, camera_pos: Point3) -> Self {
        Self {
            light_pos: [camera_pos.x, camera_pos.y, camera_pos.z, 1.0],
            world: (Matrix4::from(FIX_COORD) * world).into(),
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct OverlayUBO {
    world: RawMat4,
}

impl OverlayUBO {
    pub fn new(world: Matrix4) -> Self {
        Self {
            world: (Matrix4::from(FIX_COORD) * world).into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Face {
    pub indices: Vec<u16>,
}

impl Face {
    pub fn index_offset(mut self, offset: u16) -> Self {
        self.indices.iter_mut().for_each(|v| *v += offset);

        self
    }
}

impl PropertyAccess for Face {
    fn new() -> Self {
        Face {
            indices: Vec::new(),
        }
    }
    fn set_property(&mut self, key: String, property: Property) {
        if let ("vertex_indices", Property::ListUInt(vec)) = (key.as_ref(), property) {
            self.indices = vec.into_iter().map(|v| v as u16).collect();
        }
    }
}

// model

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, faces: Vec<Face>) -> Self {
        Self { vertices, faces }
    }
}
