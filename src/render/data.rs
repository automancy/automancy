use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix, SquareMatrix};
use hexagon_tiles::layout::{hex_to_pixel, Layout, LAYOUT_ORIENTATION_POINTY};
use hexagon_tiles::point::Point;
use ply_rs::ply::{Property, PropertyAccess};
use vulkano::impl_vertex;

use crate::game::tile::TileCoord;
use crate::render::camera::FAR;
use crate::util::cg::{Matrix3, Matrix4, Num, Point3};
use crate::util::id::Id;
use crate::util::resource::ResourceManager;

pub const RENDER_LAYOUT: Layout = Layout {
    orientation: LAYOUT_ORIENTATION_POINTY,
    size: Point { x: 1.0, y: 1.0 },
    origin: Point { x: 0.0, y: 0.0 },
};

// vertex

pub type VertexPos = [Num; 3];
pub type VertexColor = [Num; 4];

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable, Pod)]
pub struct Vertex {
    pub pos: VertexPos,
    pub color: VertexColor,
    pub normal: VertexPos,
}
impl_vertex!(Vertex, pos, color, normal);

fn color_to_num(color: u8) -> Num {
    color as Num / 255.0
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
            ("red", Property::UChar(v)) => self.color[0] = color_to_num(v),
            ("green", Property::UChar(v)) => self.color[1] = color_to_num(v),
            ("blue", Property::UChar(v)) => self.color[2] = color_to_num(v),
            ("alpha", Property::UChar(v)) => self.color[3] = color_to_num(v),
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
pub struct InstanceData {
    pub position_offset: VertexPos,
    pub scale: Num,
    pub color_offset: VertexColor,

    pub faces_index: usize,
}

impl_vertex!(InstanceData, position_offset, scale, color_offset);

impl InstanceData {
    pub fn from_id(
        id: &Id,
        pos: TileCoord,
        resource_man: Arc<ResourceManager>,
    ) -> Option<(TileCoord, Self)> {
        resource_man
            .tiles
            .get(id)
            .and_then(|r| r.faces_index)
            .map(|face| {
                let p = hex_to_pixel(RENDER_LAYOUT, pos.0);

                (
                    pos,
                    Self::new()
                        .position_offset([p.x as Num, p.y as Num, FAR as Num])
                        .faces_index(face),
                )
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

// UBO

type RawMat4 = [[Num; 4]; 4];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct GameUBO {
    pub matrix: RawMat4,
    pub ambient_light_color: [Num; 4],
    pub light_pos: VertexPos,
    pub light_color: [Num; 4],
}

impl GameUBO {
    pub fn new(matrix: Matrix4, camera_pos: Point3) -> Self {
        Self {
            matrix: matrix.into(),
            ambient_light_color: [1.0, 1.0, 1.0, 1.0],
            light_pos: camera_pos.into(),
            light_color: [1.0, 1.0, 1.0, 0.2],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct GuiUBO {
    pub matrix: RawMat4,
}

// face

#[derive(Clone, Debug)]
pub struct RawFace {
    pub indices: Vec<u32>,
}

impl RawFace {
    pub fn index_offset(mut self, offset: u32) -> Self {
        self.indices.iter_mut().for_each(|v| *v += offset);

        self
    }
}

impl PropertyAccess for RawFace {
    fn new() -> Self {
        RawFace {
            indices: Vec::new(),
        }
    }
    fn set_property(&mut self, key: String, property: Property) {
        match (key.as_ref(), property) {
            ("vertex_indices", Property::ListUInt(vec)) => {
                self.indices = vec;
            }
            (_, _) => {}
        }
    }
}

// model

#[derive(Debug, Clone)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<RawFace>,
}

impl Model {
    pub fn new(vertices: Vec<Vertex>, faces: Vec<RawFace>) -> Self {
        Self { vertices, faces }
    }
}
