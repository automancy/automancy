use std::{ops::Div, sync::Arc};

use bytemuck::{Pod, Zeroable};

use hexagon_tiles::layout::{hex_to_pixel, Layout, LAYOUT_ORIENTATION_POINTY};
use hexagon_tiles::point::Point;
use ply_rs::ply::{Property, PropertyAccess};
use vulkano::impl_vertex;

use crate::{
    data::tile::{Tile, TileCoord},
    math::{
        cg::{Num, Vector2},
    },
    registry::init::InitData,
};
use crate::render::camera::FAR;



fn color_to_num(color: u8) -> Num {
    color as Num / 255.0
}

// vertex

pub type VertexPos = [Num; 3];
pub type VertexColor = [Num; 4];

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable, Pod)]
pub struct Vertex {
    pub pos: VertexPos,
    pub color: VertexColor,
}
impl_vertex!(Vertex, pos, color);

impl PropertyAccess for Vertex {
    fn new() -> Self {
        Vertex {
            pos: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 0.0],
        }
    }

    fn set_property(&mut self, key: String, property: Property) {
        match (key.as_ref(), property) {
            ("x", Property::Float(v)) => self.pos[0] = v,
            ("y", Property::Float(v)) => self.pos[1] = v,
            ("z", Property::Float(v)) => self.pos[2] = v,
            ("red", Property::UChar(v)) => self.color[0] = color_to_num(v),
            ("green", Property::UChar(v)) => self.color[1] = color_to_num(v),
            ("blue", Property::UChar(v)) => self.color[2] = color_to_num(v),
            ("alpha", Property::UChar(v)) => self.color[3] = color_to_num(v),
            (_, _) => {}
        }
    }
}

impl Div<Vector2> for Vertex {
    type Output = Self;

    fn div(self, rhs: Vector2) -> Self::Output {
        let pos = self.pos;
        let pos = [pos[0] / rhs[0], pos[1] / rhs[1], pos[2]];
        let color = self.color;

        Self { pos, color }
    }
}

// vertex

// instance

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct InstanceData {
    pub position_offset: VertexPos,
    pub scale: Num,
    pub color_offset: VertexColor,

    pub faces_index: usize,
}

impl_vertex!(InstanceData, position_offset, scale, color_offset);

pub const RENDER_LAYOUT: Layout = Layout {
    orientation: LAYOUT_ORIENTATION_POINTY,
    size: Point { x: 1.0, y: 1.0 },
    origin: Point { x: 0.0, y: 0.0 },
};

impl InstanceData {
    pub fn from_tile(tile: &Tile, pos: TileCoord, init_data: Arc<InitData>) -> Option<(TileCoord, Self)> {
        init_data
            .resource_man
            .resources
            .get(&tile.id)
            .and_then(|r| r.faces_index)
            .map(|face| {
                let p = hex_to_pixel(RENDER_LAYOUT, pos.0);

                (pos, Self::new().position_offset([p.x as Num, p.y as Num, FAR as Num]).faces_index(face))
            })
    }

    pub fn new() -> Self {
        InstanceData {
            position_offset: [0.0, 0.0, 0.0],
            scale: 1.0,
            faces_index: 0,
            color_offset: [0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn position_offset(mut self, position_offset: VertexPos) -> Self {
        self.position_offset = position_offset;

        self
    }

    pub fn add_position_offset(mut self, position_offset: VertexPos) -> Self {
        self.position_offset[0] += position_offset[0];
        self.position_offset[1] += position_offset[1];
        self.position_offset[2] += position_offset[2];

        self
    }

    pub fn scale(mut self, scale: Num) -> Self {
        self.scale = scale;

        self
    }

    pub fn faces_index(mut self, faces_index: usize) -> Self {
        self.faces_index = faces_index;

        self
    }

    pub fn color_offset(mut self, color_offset: VertexColor) -> Self {
        self.color_offset = color_offset;

        self
    }
}

// instance

// UBO

type RawMat4 = [[Num; 4]; 4];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct UniformBufferObject {
    pub matrix: RawMat4,
}

// UBO

// face

#[derive(Clone, Debug)]
pub struct Face {
    pub indices: Vec<u32>,
    pub offset: Option<u32>,
}

impl Face {
    pub fn index_offset(&mut self, offset: u32) {
        self.indices.iter_mut().for_each(|v| *v += offset);
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);

        self
    }
}

impl PropertyAccess for Face {
    fn new() -> Self {
        Face {
            indices: Vec::new(),
            offset: None,
        }
    }
    fn set_property(&mut self, key: String, property: Property) {
        match (key.as_ref(), property) {
            ("vertex_indices", Property::ListUInt(vec)) => {
                self.indices = vec;
            },
            (_, _) => {}
        }
    }
}

// face

// model

#[derive(Debug, Clone)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
}

impl Model {
    pub fn new(vertices: Vec<Vertex>, faces: Vec<Face>) -> Self {
        Self { vertices, faces }
    }
}

// model
