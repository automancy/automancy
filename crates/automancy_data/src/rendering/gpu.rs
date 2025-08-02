use crate::{
    math::{Float, Matrix4, Vec3, Vec4},
    rendering::colors::Rgba,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instance {
    pub color_offset: Rgba,
    pub alpha: Float,
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            color_offset: Rgba::zero(),
            alpha: 1.0,
        }
    }
}

impl Instance {
    #[inline]
    pub fn add_alpha(mut self, alpha: Float) -> Self {
        self.alpha *= alpha;

        self
    }

    #[inline]
    pub fn with_alpha(mut self, alpha: Float) -> Self {
        self.alpha = alpha;

        self
    }

    #[inline]
    pub fn with_color_offset(mut self, color_offset: Rgba) -> Self {
        self.color_offset = color_offset;

        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GameMatrixData<const HAS_MESH_MATRIX: bool> {
    model_matrix: Matrix4,
    world_matrix: Matrix4,
    mesh_matrix: Option<Matrix4>,
}

impl GameMatrixData<true> {
    pub fn new(
        model_matrix: Matrix4,
        world_matrix: Matrix4,
        mesh_matrix: Matrix4,
    ) -> GameMatrixData<true> {
        GameMatrixData {
            model_matrix,
            world_matrix,
            mesh_matrix: Some(mesh_matrix),
        }
    }

    pub fn model_matrix(&self) -> Matrix4 {
        self.model_matrix
    }
    pub fn world_matrix(&self) -> Matrix4 {
        self.world_matrix
    }
    pub fn mesh_matrix(&self) -> Matrix4 {
        unsafe { self.mesh_matrix.unwrap_unchecked() }
    }
}

impl GameMatrixData<false> {
    pub fn new(model_matrix: Matrix4, world_matrix: Matrix4) -> GameMatrixData<false> {
        GameMatrixData {
            model_matrix,
            world_matrix,
            mesh_matrix: None,
        }
    }

    pub fn model_matrix(&self) -> Matrix4 {
        self.model_matrix
    }
    pub fn world_matrix(&self) -> Matrix4 {
        self.world_matrix
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnimationMatrixData {
    pub animation_matrix: Matrix4,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GameUniformData {
    light_color: Rgba,
    light_pos: Vec4,
}

impl GameUniformData {
    pub fn new(light_pos: Vec3, light_strength: Option<Float>) -> Self {
        Self {
            light_color: Rgba::white(),
            light_pos: light_pos.with_w(light_strength.unwrap_or(1.0)),
        }
    }
}

impl Default for GameUniformData {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 0.0, 6.0), None)
    }
}

pub const FLAG_SCREEN_EFFECT: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PostProcessingUniformData {
    pub flags: u32,
    pub _p0: [f32; 3],
}

impl Default for PostProcessingUniformData {
    fn default() -> Self {
        Self {
            flags: FLAG_SCREEN_EFFECT,
            _p0: [0.0; 3],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IntermediateUniformData {
    pub viewport_size: [f32; 2],
    pub viewport_pos: [f32; 2],
}

impl Default for IntermediateUniformData {
    fn default() -> Self {
        Self {
            viewport_size: [1.0, 1.0],
            viewport_pos: [0.0, 0.0],
        }
    }
}
