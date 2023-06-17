use crate::cg::{Float, Matrix4, Point3, Vector3};
use bytemuck::{Pod, Zeroable};
use cgmath::SquareMatrix;
use egui::ecolor::{linear_f32_from_gamma_u8, linear_f32_from_linear_u8};
use hexagon_tiles::layout::{Layout, LAYOUT_ORIENTATION_POINTY};
use hexagon_tiles::point::Point;
use ply_rs::ply::{Property, PropertyAccess};
use vulkano::pipeline::graphics::vertex_input::Vertex;

pub const HEX_GRID_LAYOUT: Layout = Layout {
    orientation: LAYOUT_ORIENTATION_POINTY,
    size: Point { x: 1.0, y: 1.0 },
    origin: Point { x: 0.0, y: 0.0 },
};

// vertex

pub type VertexPos = [Float; 3];
pub type VertexColor = [Float; 4];
pub type RawMat4 = [[Float; 4]; 4];

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable, Pod, Vertex)]
pub struct GameVertex {
    #[format(R32G32B32_SFLOAT)]
    pub pos: VertexPos,
    #[format(R32G32B32A32_SFLOAT)]
    pub color: VertexColor,
    #[format(R32G32B32_SFLOAT)]
    pub normal: VertexPos,
}

impl PropertyAccess for GameVertex {
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
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod, Vertex)]
pub struct RawInstanceData {
    #[format(R32G32B32A32_SFLOAT)]
    pub color_offset: VertexColor,
    #[format(R32G32B32A32_SFLOAT)]
    pub model_matrix: RawMat4,
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod, Vertex)]
pub struct LightInfo {
    #[format(R32G32B32_SFLOAT)]
    pub light_pos: VertexPos,
    #[format(R32G32B32A32_SFLOAT)]
    pub light_color: VertexColor,
}

// UBO

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct GameUBO {
    pub matrix: RawMat4,
    pub light: LightInfo,
}

pub static DEFAULT_LIGHT_COLOR: VertexColor = [0.9, 0.9, 0.9, 0.9];

impl GameUBO {
    pub fn new(matrix: Matrix4, camera_pos: Point3) -> Self {
        Self {
            matrix: matrix.into(),
            light: LightInfo {
                light_pos: camera_pos.into(),
                light_color: DEFAULT_LIGHT_COLOR,
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct OverlayUBO {
    pub matrix: RawMat4,
}

#[derive(Clone, Debug)]
pub struct Face {
    pub indices: Vec<u32>,
}

impl Face {
    pub fn index_offset(mut self, offset: u32) -> Self {
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
            self.indices = vec;
        }
    }
}

// model

#[derive(Debug, Clone)]
pub struct Model {
    pub vertices: Vec<GameVertex>,
    pub faces: Vec<Face>,
}

impl Model {
    pub fn new(vertices: Vec<GameVertex>, faces: Vec<Face>) -> Self {
        Self { vertices, faces }
    }
}
