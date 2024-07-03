use yakui::Vec2;

pub const HEADING_SIZE: f32 = 20.0;
pub const LABEL_SIZE: f32 = 14.0;
pub const SMALL_SIZE: f32 = 12.0;
pub const DIVIER_SIZE: f32 = 1.0;
pub const ICON_SIZE: Vec2 = Vec2::new(12.0, 12.0);

pub const PADDING_SMALL: f32 = 4.0;
pub const PADDING_MEDIUM: f32 = 8.0;
pub const PADDING_LARGE: f32 = 12.0;
pub const PADDING_XLARGE: f32 = 16.0;

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
