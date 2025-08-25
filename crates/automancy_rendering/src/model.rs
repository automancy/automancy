use automancy_data::math::{Float, Matrix4};
use bytemuck::{Pod, Zeroable};

use crate::data::GpuVertex;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct MeshId(u16);

impl From<u16> for MeshId {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct GlobalMeshId(u32);

impl From<u32> for GlobalMeshId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub index: MeshId,

    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u16>,

    pub opaque: bool,
    pub transform: gltf::scene::Transform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub target: MeshId,
    pub interpolation: gltf::animation::Interpolation,
    pub inputs: Vec<Float>,
    pub outputs: Vec<Matrix4>,
}
