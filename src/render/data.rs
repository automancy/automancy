use std::sync::Arc;

use crate::game::tile::coord::TileCoord;
use crate::game::tile::entity::TileState;
use bytemuck::{Pod, Zeroable};
use cgmath::{vec3, SquareMatrix};
use egui::ecolor::{linear_f32_from_gamma_u8, linear_f32_from_linear_u8};
use hexagon_tiles::layout::{hex_to_pixel, Layout, LAYOUT_ORIENTATION_POINTY};
use hexagon_tiles::point::Point;
use ply_rs::ply::{Property, PropertyAccess};
use vulkano::pipeline::graphics::vertex_input::Vertex;

use crate::render::camera::FAR;
use crate::resource::ResourceManager;
use crate::util::cg::{Float, Matrix4, Point3, Vector3};
use crate::util::id::Id;

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
    pub fn from_tile(
        resource_man: Arc<ResourceManager>,
        id: Id,
        pos: TileCoord,
        tile_state: TileState,
    ) -> Option<(TileCoord, (Self, Id))> {
        resource_man
            .registry
            .get_tile(id)
            .and_then(|r| r.models.get(tile_state as usize).cloned())
            .map(|model| {
                let p = hex_to_pixel(HEX_GRID_LAYOUT, pos.into());

                (
                    pos,
                    (
                        Self::default().add_translation(vec3(
                            p.x as Float,
                            p.y as Float,
                            FAR as Float,
                        )),
                        model,
                    ),
                )
            })
    }

    pub fn add_translation(mut self, translation: Vector3) -> Self {
        self.model_matrix = self.model_matrix * Matrix4::from_translation(translation);

        self
    }

    pub fn add_scale(mut self, scale: Float) -> Self {
        self.model_matrix = self.model_matrix * Matrix4::from_scale(scale);

        self
    }

    pub fn with_color_offset(mut self, color_offset: VertexColor) -> Self {
        self.color_offset = color_offset;

        self
    }
}

// UBO

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct GameUBO {
    pub matrix: RawMat4,
    pub light_pos: VertexPos,
    pub light_color: [Float; 4],
}

impl GameUBO {
    pub fn new(matrix: Matrix4, camera_pos: Point3) -> Self {
        Self {
            matrix: matrix.into(),
            light_pos: camera_pos.into(),
            light_color: [1.0, 1.0, 1.0, 0.9],
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
        if let ("vertex_indices", Property::ListUInt(vec)) = (key.as_ref(), property) {
            self.indices = vec;
        }
    }
}

// model

#[derive(Debug, Clone)]
pub struct Model {
    pub vertices: Vec<GameVertex>,
    pub faces: Vec<RawFace>,
}

impl Model {
    pub fn new(vertices: Vec<GameVertex>, faces: Vec<RawFace>) -> Self {
        Self { vertices, faces }
    }
}
