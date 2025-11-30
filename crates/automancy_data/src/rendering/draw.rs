use core::fmt::Display;

use bytemuck::{ByteEq, ByteHash, Pod, Zeroable};

use crate::{
    math::{Float, Matrix4, Rect, Vec3},
    rendering::{
        colors,
        colors::{PackedRgba, Rgba},
    },
};

#[must_use]
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ByteHash, ByteEq)]
pub struct GameDrawInstance {
    pub color_offset: PackedRgba,
    pub alpha: Float,
    pub model_matrix: Matrix4,
    pub world_matrix: Matrix4,
}

impl Default for GameDrawInstance {
    fn default() -> Self {
        Self {
            color_offset: colors::TRANSPARENT.packed,
            alpha: 1.0,
            model_matrix: Matrix4::identity(),
            world_matrix: Matrix4::identity(),
        }
    }
}

impl Display for GameDrawInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "({}, {}, <model>, <world>)",
            const_hex::encode(self.color_offset.to_be_bytes()),
            self.alpha
        ))
    }
}

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct AnimationMatrixData {
    pub animation_matrix: Matrix4,
}

#[must_use]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GameUniformData {
    pub camera_pos: Vec3,
    pub light_color: Rgba,
    pub light_strength: Float,
    pub camera_bounds: Rect,
    pub view_matrix: Matrix4,
}

impl Default for GameUniformData {
    fn default() -> Self {
        Self {
            camera_pos: Vec3::new(0.0, 0.0, 1.0),
            light_color: Rgba::white(),
            light_strength: 1.0,
            camera_bounds: Rect::default(),
            view_matrix: Matrix4::default(),
        }
    }
}
