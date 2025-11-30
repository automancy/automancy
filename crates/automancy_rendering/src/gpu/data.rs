use automancy_data::{
    math::Matrix4,
    rendering::{colors::PackedRgba, draw::GameUniformData},
};
use bytemuck::{ByteEq, ByteHash, Pod, Zeroable};

use crate::MeshId;

#[allow(unused)]
pub(crate) type GpuPos3 = [f32; 3];
#[allow(unused)]
pub(crate) type GpuPos4 = [f32; 4];
#[allow(unused)]
pub(crate) type GpuColor = [f32; 4];

// memory alignment issue, padded to 16 bytes
#[allow(unused)]
pub(crate) type GpuMat3 = [[f32; 4]; 3];
#[allow(unused)]
pub(crate) type GpuMat4 = [[f32; 4]; 4];

#[allow(unused)]
const IDENTITY_MAT_3: GpuMat3 = [
    [1.0, 0.0, 0.0, 0.0], //
    [0.0, 1.0, 0.0, 0.0], //
    [0.0, 0.0, 1.0, 0.0], //
];

#[allow(unused)]
const IDENTITY_MAT_4: GpuMat4 = [
    [1.0, 0.0, 0.0, 0.0], //
    [0.0, 1.0, 0.0, 0.0], //
    [0.0, 0.0, 1.0, 0.0], //
    [0.0, 0.0, 0.0, 1.0], //
];

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialOrd, Pod, Zeroable, ByteHash, ByteEq)]

pub struct GpuGameVertex {
    pub pos: GpuPos3,
    pub normal: GpuPos3,
    pub color: GpuColor,
}

impl GpuGameVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        static ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x4,
        ];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuGameDrawInstance {
    pub mesh_id: MeshId,
    pub color_offset: PackedRgba,
    pub alpha: f32,
    pub model_matrix_index: u32,
    pub world_matrix_index: u16,
    pub animation_index: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuGameMeshData {
    pub mesh_matrix: GpuMat4,
}

impl GpuGameMeshData {
    pub fn new(matrix: Matrix4) -> Self {
        GpuGameMeshData {
            mesh_matrix: matrix.into_col_arrays(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuGameModelMatrixData {
    pub model_matrix: GpuMat4,
}

impl GpuGameModelMatrixData {
    pub fn new(matrix: Matrix4) -> Self {
        GpuGameModelMatrixData {
            model_matrix: matrix.into_col_arrays(),
        }
    }
}

impl Default for GpuGameModelMatrixData {
    fn default() -> Self {
        Self {
            model_matrix: IDENTITY_MAT_4,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuGameWorldMatrixData {
    pub world_matrix: GpuMat4,
}

impl GpuGameWorldMatrixData {
    pub fn new(matrix: Matrix4) -> Self {
        Self {
            world_matrix: matrix.into_col_arrays(),
        }
    }
}

impl Default for GpuGameWorldMatrixData {
    fn default() -> Self {
        Self {
            world_matrix: IDENTITY_MAT_4,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuGameAnimationMatrixData {
    animation_matrix: GpuMat4,
}

impl GpuGameAnimationMatrixData {
    pub fn new(matrix: Matrix4) -> Self {
        Self {
            animation_matrix: matrix.into_col_arrays(),
        }
    }
}

impl Default for GpuGameAnimationMatrixData {
    fn default() -> Self {
        Self {
            animation_matrix: IDENTITY_MAT_4,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuGameUniformData {
    camera_pos: GpuPos3,
    // padding
    __1: [f32; 1],
    camera_bounds: GpuPos4,
    view_matrix: GpuMat4,
}

impl GpuGameUniformData {
    pub fn new(uniform: &GameUniformData) -> Self {
        Self {
            camera_pos: uniform.camera_pos.into_array(),
            __1: [0.0; 1],
            camera_bounds: [
                uniform.camera_bounds.min.x,
                uniform.camera_bounds.min.y,
                uniform.camera_bounds.max.x,
                uniform.camera_bounds.max.y,
            ],
            view_matrix: uniform.view_matrix.into_col_arrays(),
        }
    }
}

impl Default for GpuGameUniformData {
    fn default() -> Self {
        Self::new(&GameUniformData::default())
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuGameLightingUniformData {
    light_color: GpuColor,
    light_strength: f32,
    // padding
    __1: [f32; 3],
}

impl GpuGameLightingUniformData {
    pub fn new(uniform: &GameUniformData) -> Self {
        Self {
            light_color: uniform.light_color.into_array(),
            light_strength: uniform.light_strength,
            __1: [0.0; 3],
        }
    }
}

impl Default for GpuGameLightingUniformData {
    fn default() -> Self {
        Self::new(&GameUniformData::default())
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuPostProcessingUniformData {
    pub flags: u32,
    pub _p0: [f32; 3],
}

impl GpuPostProcessingUniformData {
    pub const FLAG_NONE: u32 = 0;
    pub const FLAG_SCREEN_EFFECT: u32 = 1;
}

impl Default for GpuPostProcessingUniformData {
    fn default() -> Self {
        Self {
            flags: Self::FLAG_SCREEN_EFFECT,
            _p0: [0.0; 3],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuBlitDrawInstance {
    pub viewport_size: [f32; 2],
    pub viewport_pos: [f32; 2],
}

impl Default for GpuBlitDrawInstance {
    fn default() -> Self {
        Self {
            viewport_size: [1.0, 1.0],
            viewport_pos: [0.0, 0.0],
        }
    }
}

impl GpuBlitDrawInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        static ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
        ];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: ATTRIBUTES,
        }
    }
}
