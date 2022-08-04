use std::ops::Div;

use bytemuck::{Pod, Zeroable};
use ply_rs::ply::{Property, PropertyAccess};
use vulkano::impl_vertex;

use crate::math::data::{Num, Vector2};

fn color_to_num(color: u8) -> Num {
    color as Num / 255.0
}

// vertex

pub type VertexPos = [Num; 3];
pub type VertexColor = [Num; 4];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
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
    pub faces_index: usize,
}
impl_vertex!(InstanceData, position_offset, scale);

// instance

// UBO

type RawMat4 = [[Num; 4]; 4];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct UniformBufferObject {
    pub view: RawMat4,
    //pub projection: RawMat4,
}

// UBO

// face

#[derive(Clone, Debug)]
pub struct Face {
    pub vertex_indices: Vec<u32>,
}

impl PropertyAccess for Face {
    fn new() -> Self {
        Face {
            vertex_indices: Vec::new(),
        }
    }
    fn set_property(&mut self, key: String, property: Property) {
        match (key.as_ref(), property) {
            ("vertex_indices", Property::ListUInt(vec)) => self.vertex_indices = vec,
            (_, _) => {}
        }
    }
}

// face

// model

#[derive(Clone, Debug)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
}

// model

/*
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: Num,
    pub g: Num,
    pub b: Num,
    pub a: Num,
}

impl From<[Num; 3]> for Color {
    fn from(n: [Num; 3]) -> Self {
        Self {
            r: n[0],
            g: n[1],
            b: n[2],
            a: 1.0,
        }
    }
}

impl From<[Num; 4]> for Color {
    fn from(n: [Num; 4]) -> Self {
        Self {
            r: n[0],
            g: n[1],
            b: n[2],
            a: n[3],
        }
    }
}
*/
