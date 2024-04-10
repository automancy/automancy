use automancy_defs::{
    glam::vec2,
    math::{Float, Vec2},
};

pub const HEADING_SIZE: Float = 24.0;
pub const TEXT_SIZE: Float = 16.0;
pub const DIVIER_SIZE: Float = 1.0;
pub const ICON_SIZE: Vec2 = vec2(12.0, 12.0);

pub const PADDING_MEDIUM: Float = 8.0;

pub mod absolute;
pub mod button;
pub mod container;
pub mod hover;
pub mod interactive;
pub mod list;
pub mod position;
pub mod select;
pub mod text;
pub mod window;
