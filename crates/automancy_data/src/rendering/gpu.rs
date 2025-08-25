use crate::{
    math::{Float, Matrix4, Vec3, Vec4},
    rendering::colors::Rgba,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameDrawInstance {
    pub color_offset: Rgba,
    pub alpha: Float,
}

impl Default for GameDrawInstance {
    fn default() -> Self {
        Self {
            color_offset: Rgba::zero(),
            alpha: 1.0,
        }
    }
}

impl GameDrawInstance {
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
pub struct GameMatrixData {
    model_matrix: Matrix4,
    world_matrix: Matrix4,
}

impl GameMatrixData {
    pub fn new(model_matrix: Matrix4, world_matrix: Matrix4) -> GameMatrixData {
        GameMatrixData {
            model_matrix,
            world_matrix,
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
