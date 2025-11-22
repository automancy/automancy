use automancy_data::{
    math::{Float, Matrix4},
    rendering::{colors::Rgba, draw::GameUniformData},
};
use bytemuck::{ByteEq, ByteHash, Pod, Zeroable};

#[allow(unused)]
pub(crate) type GpuPos3 = [Float; 3];
#[allow(unused)]
pub(crate) type GpuPos4 = [Float; 4];
#[allow(unused)]
pub(crate) type GpuColor = [Float; 4];

// memory alignment issue, padded to 16 bytes
#[allow(unused)]
pub(crate) type GpuMat3 = [[Float; 4]; 3];
#[allow(unused)]
pub(crate) type GpuMat4 = [[Float; 4]; 4];

#[allow(unused)]
const IDENTITY_MAT_3: GpuMat3 = [
    [1.0, 0.0, 0.0, 0.0], //
    [0.0, 1.0, 0.0, 0.0], //
    [0.0, 0.0, 1.0, 0.0], //
];

const IDENTITY_MAT_4: GpuMat4 = [
    [1.0, 0.0, 0.0, 0.0], //
    [0.0, 1.0, 0.0, 0.0], //
    [0.0, 0.0, 1.0, 0.0], //
    [0.0, 0.0, 0.0, 1.0], //
];

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialOrd, Pod, Zeroable, ByteHash, ByteEq)]

pub struct GpuVertex {
    pub pos: GpuPos3,
    pub normal: GpuPos3,
    pub color: GpuColor,
}

impl GpuVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        static ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x4,
        ];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuDrawInstance {
    pub color_offset: GpuColor,
    pub alpha: Float,
    pub model_matrix_index: u32,
    pub world_matrix_index: u16,
    pub animation_index: u16,
}

impl GpuDrawInstance {
    pub fn new(color_offset: Rgba, alpha: Float, model_matrix_index: u32, world_matrix_index: u16, animation_index: u16) -> Self {
        Self {
            color_offset: color_offset.into_array(),
            alpha,
            model_matrix_index,
            world_matrix_index,
            animation_index,
        }
    }
}

impl GpuDrawInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        static ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            3 => Float32x4,
            4 => Float32,
            5 => Uint32,
            6 => Uint16,
            7 => Uint16,
        ];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<GpuDrawInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuModelMatrixData {
    pub model_matrix: GpuMat4,
}

impl GpuModelMatrixData {
    pub fn new(matrix: Matrix4) -> Self {
        GpuModelMatrixData {
            model_matrix: matrix.into_col_arrays(),
        }
    }
}

impl Default for GpuModelMatrixData {
    fn default() -> Self {
        Self {
            model_matrix: IDENTITY_MAT_4,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuWorldMatrixData {
    pub world_matrix: GpuMat4,
}

impl GpuWorldMatrixData {
    pub fn new(matrix: Matrix4) -> Self {
        Self {
            world_matrix: matrix.into_col_arrays(),
        }
    }
}

impl Default for GpuWorldMatrixData {
    fn default() -> Self {
        Self {
            world_matrix: IDENTITY_MAT_4,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuAnimationMatrixData {
    animation_matrix: GpuMat4,
}

impl GpuAnimationMatrixData {
    pub fn new(matrix: Matrix4) -> Self {
        Self {
            animation_matrix: matrix.into_col_arrays(),
        }
    }
}

impl Default for GpuAnimationMatrixData {
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
    light_strength: Float,
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

pub const FLAG_SCREEN_EFFECT: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuPostProcessingUniformData {
    pub flags: u32,
    pub _p0: [f32; 3],
}

impl Default for GpuPostProcessingUniformData {
    fn default() -> Self {
        Self {
            flags: FLAG_SCREEN_EFFECT,
            _p0: [0.0; 3],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GpuTextureSampleUniformData {
    pub viewport_size: [f32; 2],
    pub viewport_pos: [f32; 2],
}

impl Default for GpuTextureSampleUniformData {
    fn default() -> Self {
        Self {
            viewport_size: [1.0, 1.0],
            viewport_pos: [0.0, 0.0],
        }
    }
}
