use automancy_defs::{
    glam::vec2,
    math::{Float, Vec2},
};

pub const HEADING_SIZE: Float = 24.0;
pub const TEXT_SIZE: Float = 16.0;
pub const DIVIER_SIZE: Float = 1.0;
pub const ICON_SIZE: Vec2 = vec2(12.0, 12.0);

pub const PADDING_MEDIUM: Float = 8.0;
pub const PADDING_LARGE: Float = 12.0;

pub mod absolute;
pub mod button;
pub mod checkbox;
pub mod container;
pub mod hover;
pub mod interactive;
pub mod layer;
pub mod layout;
pub mod position;
pub mod relative;
pub mod scrollable;
pub mod select;
pub mod slider;
pub mod text;
pub mod window;
