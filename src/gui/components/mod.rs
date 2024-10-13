use yakui::Vec2;

pub const HEADING_SIZE: f32 = 20.0;
pub const LABEL_SIZE: f32 = 14.0;
pub const SMALL_SIZE: f32 = 12.0;
pub const DIVIER_THICKNESS: f32 = 2.0;
pub const DIVIER_HEIGHT: f32 = 6.0;
pub const ICON_SIZE: Vec2 = Vec2::new(12.0, 12.0);

pub const PADDING_XSMALL: f32 = 2.0;
pub const PADDING_SMALL: f32 = 4.0;
pub const PADDING_MEDIUM: f32 = 8.0;
pub const PADDING_LARGE: f32 = 12.0;
pub const PADDING_XLARGE: f32 = 16.0;

mod button;
mod checkbox;
mod container;
mod game_object;
mod hover;
mod interactive;
mod layout;
mod movable;
mod position;
mod scrollable;
mod select;
mod slider;
mod text;
mod textbox;
mod tip;

pub use self::button::*;
pub use self::checkbox::*;
pub use self::container::*;
pub use self::game_object::*;
pub use self::hover::*;
pub use self::interactive::*;
pub use self::layout::*;
pub use self::movable::*;
pub use self::position::*;
pub use self::scrollable::*;
pub use self::select::*;
pub use self::slider::*;
pub use self::text::*;
pub use self::textbox::*;
pub use self::tip::*;
