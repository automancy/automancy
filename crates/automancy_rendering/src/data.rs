use automancy_data::{
    math::{Float, Matrix3, Matrix4, Vec3},
    rendering::colors::Rgba,
};
use bytemuck::{ByteEq, ByteHash, Pod, Zeroable};

pub type GpuPos3 = [Float; 3];
pub type GpuPos4 = [Float; 4];
pub type GpuColor = [Float; 4];

// memory alignment issue, padded to 16 bytes
pub type GpuMat3 = [[Float; 4]; 3];
pub type GpuMat4 = [[Float; 4]; 4];

const IDENTITY_3: GpuMat3 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

const IDENTITY_4: GpuMat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
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
#[derive(Debug, Clone, Copy, Default, ByteEq, Pod, Zeroable, ByteHash)]
pub struct GpuDrawInstance {
    pub color_offset: GpuColor,
    pub alpha: Float,
    pub matrix_index: u32,
    pub world_matrix_index: u32,
    pub animation_matrix_index: u32,
}

impl GpuDrawInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        static ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            3 => Float32x4,
            4 => Float32,
            5 => Uint32,
            6 => Uint32,
            7 => Uint32,
        ];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<GpuDrawInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuGameMatrixData {
    pub model_matrix: GpuMat4,
    pub normal_matrix: GpuMat3,
}

impl Default for GpuGameMatrixData {
    fn default() -> Self {
        Self {
            model_matrix: IDENTITY_4,
            normal_matrix: IDENTITY_3,
        }
    }
}

impl GpuGameMatrixData {
    pub fn new(model_matrix: Matrix4, mesh_matrix: Matrix4) -> Self {
        let matrix = model_matrix * mesh_matrix;

        let inverse_transpose = Matrix3::from(matrix.inverted()).transposed();

        GpuGameMatrixData {
            model_matrix: matrix.into_col_arrays(),
            normal_matrix: [
                inverse_transpose.cols.x.with_w(0.0).into_array(),
                inverse_transpose.cols.y.with_w(0.0).into_array(),
                inverse_transpose.cols.z.with_w(0.0).into_array(),
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuAnimationMatrixData {
    pub animation_matrix: GpuMat4,
}

impl Default for GpuAnimationMatrixData {
    fn default() -> Self {
        Self {
            animation_matrix: IDENTITY_4,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuWorldMatrixData {
    world_matrix: GpuMat4,
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
            world_matrix: IDENTITY_4,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuGameUniformData {
    light_color: GpuColor,
    light_pos: GpuPos4,
}

impl GpuGameUniformData {
    pub fn new(light_color: Rgba, light_pos: Vec3, light_strength: Float) -> Self {
        let light_pos = light_pos.into_array();

        Self {
            light_color: light_color.into_array(),
            light_pos: [light_pos[0], light_pos[1], light_pos[2], light_strength],
        }
    }
}

impl Default for GpuGameUniformData {
    fn default() -> Self {
        Self::new(Rgba::white(), Vec3::new(0.0, 0.0, 6.0), 1.0)
    }
}
