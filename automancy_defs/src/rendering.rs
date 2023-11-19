use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use cgmath::{point3, vec3, EuclideanSpace, MetricSpace, SquareMatrix};
use egui::NumExt;
use gltf::animation::Interpolation;
use hexagon_tiles::fractional::FractionalHex;
use hexagon_tiles::traits::HexRound;
use wgpu::{vertex_attr_array, BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::coord::TileCoord;
use crate::math;
use crate::math::{
    direction_to_angle, DPoint2, Double, Float, Matrix4, Point3, Quaternion, Vector3,
};

pub fn lerp_coords_to_pixel(a: TileCoord, b: TileCoord, t: Double) -> DPoint2 {
    let a = FractionalHex::new(a.q() as Double, a.r() as Double);
    let b = FractionalHex::new(b.q() as Double, b.r() as Double);
    let lerp = FractionalHex::lerp(a, b, t);

    math::frac_hex_to_pixel(lerp)
}

/// Produces a line shape.
pub fn make_line(a: DPoint2, b: DPoint2) -> Matrix4 {
    let mid = a.midpoint(b);
    let d = a.distance(b) as Float;
    let theta = direction_to_angle(b - a);

    Matrix4::from_translation(vec3(mid.x as Float, mid.y as Float, 0.1))
        * Matrix4::from_angle_z(theta)
        * Matrix4::from_nonuniform_scale(d.at_least(0.001), 0.1, 0.05)
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
    pub color_offset: VertexColor,
    pub alpha: Float,
    pub light_pos: Point3,
    pub model_matrix: Matrix4,
}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            color_offset: [0.0, 0.0, 0.0, 0.0],
            alpha: 1.0,
            light_pos: point3(0.0, 0.0, 8.0),
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
    pub fn add_alpha(mut self, alpha: Float) -> Self {
        self.alpha *= alpha;

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
    pub fn with_alpha(mut self, alpha: Float) -> Self {
        self.alpha = alpha;

        self
    }

    #[inline]
    pub fn with_light_pos(mut self, light_pos: Point3) -> Self {
        self.light_pos = light_pos;

        self
    }

    #[inline]
    pub fn with_color_offset(mut self, color_offset: VertexColor) -> Self {
        self.color_offset = color_offset;

        self
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct RawInstanceData {
    color_offset: VertexColor,
    alpha: Float,
    light_pos: VertexPos,
    model_matrix: RawMat4,
}

impl From<InstanceData> for RawInstanceData {
    fn from(value: InstanceData) -> Self {
        Self {
            color_offset: value.color_offset,
            alpha: value.alpha,
            light_pos: [value.light_pos.x, value.light_pos.y, value.light_pos.z],
            model_matrix: value.model_matrix.into(),
        }
    }
}

impl RawInstanceData {
    pub fn desc() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array![
            3 => Float32x4,
            4 => Float32,
            5 => Float32x3,
            6 => Float32x4,
            7 => Float32x4,
            8 => Float32x4,
            9 => Float32x4,
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
    world_matrix: RawMat4,
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
            light_color: DEFAULT_LIGHT_COLOR,
            world_matrix: FIX_COORD,
        }
    }
}

impl GameUBO {
    pub fn new(world: Matrix4) -> Self {
        let world = Matrix4::from(FIX_COORD) * world;

        Self {
            world_matrix: world.into(),
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct PostEffectsUBO {
    pub _empty: Float,
}

// model

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub name: String,
    pub index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationUnit {
    Translation(Vector3),
    Scale(Vector3),
    Rotate(Quaternion),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub target: usize,
    pub min: Float,
    pub max: Float,
    pub interpolation: Interpolation,
    pub inputs: Vec<Float>,
    pub outputs: Vec<AnimationUnit>,
}
