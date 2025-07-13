use crate::{
    math::{Float, Matrix4, Vec3},
    rendering::colors::Rgba,
};

pub const NO_COLOR_OFFSET: Rgba = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameDrawInstance {
    pub color_offset: Rgba,
    pub alpha: Float,
    pub model_matrix: Matrix4,
    pub world_matrix: Matrix4,
}

impl Default for GameDrawInstance {
    fn default() -> Self {
        Self {
            color_offset: NO_COLOR_OFFSET,
            alpha: 1.0,
            model_matrix: Matrix4::identity(),
            world_matrix: Matrix4::identity(),
        }
    }
}

impl GameDrawInstance {
    #[inline]
    pub fn mul_alpha(mut self, alpha: Float) -> Self {
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
pub struct AnimationMatrixData {
    pub animation_matrix: Matrix4,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GameUniformData {
    pub camera_pos: Vec3,
    pub light_color: Rgba,
    pub light_strength: Float,
}

impl Default for GameUniformData {
    fn default() -> Self {
        Self {
            camera_pos: Vec3::new(0.0, 0.0, 6.0),
            light_color: Rgba::white(),
            light_strength: 1.0,
        }
    }
}
