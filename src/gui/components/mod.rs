use automancy_defs::{
    glam::vec2,
    math::{Float, Vec2},
};

pub const HEADING_SIZE: Float = 20.0;
pub const LABEL_SIZE: Float = 14.0;
pub const SMALL_SIZE: Float = 12.0;
pub const DIVIER_SIZE: Float = 1.0;
pub const ICON_SIZE: Vec2 = vec2(12.0, 12.0);

pub const PADDING_SMALL: Float = 4.0;
pub const PADDING_MEDIUM: Float = 8.0;
pub const PADDING_LARGE: Float = 12.0;
pub const PADDING_XLARGE: Float = 16.0;

mod button;
mod checkbox;
mod container;
mod game;
mod hover;
mod interactive;
mod layout;
mod movable;
mod position;
mod relative;
mod scrollable;
mod select;
mod slider;
mod text;
mod textbox;
mod tip;

pub use self::button::*;
pub use self::checkbox::*;
pub use self::container::*;
pub use self::game::*;
pub use self::hover::*;
pub use self::interactive::*;
pub use self::layout::*;
pub use self::movable::*;
pub use self::position::*;
pub use self::relative::*;
pub use self::scrollable::*;
pub use self::select::*;
pub use self::slider::*;
pub use self::text::*;
pub use self::textbox::*;
pub use self::tip::*;
