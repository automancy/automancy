use crate::{
    math::{Float, Matrix4, Vec2, Vec3},
    rendering::colors::Rgba,
};

pub const NO_COLOR_OFFSET: Rgba = Rgba::broadcast(0.0);

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
    pub camera_bounds: (Vec2, Vec2),
}

impl Default for GameUniformData {
    fn default() -> Self {
        Self {
            camera_pos: Vec3::new(0.0, 0.0, 6.0),
            light_color: Rgba::white(),
            light_strength: 1.0,
            camera_bounds: (Vec2::zero(), Vec2::one()),
        }
    }
}
