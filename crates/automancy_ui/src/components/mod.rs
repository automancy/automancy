use automancy_defs::math::Vec2;

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
mod shapes;
mod slider;
mod text;
mod textbox;
mod tip;
mod util;

pub use self::{
    button::*, checkbox::*, container::*, game_object::*, hover::*, interactive::*, layout::*,
    movable::*, position::*, scrollable::*, select::*, shapes::*, slider::*, text::*, textbox::*,
    tip::*, util::*,
};

pub mod custom {
    use core::cell::Cell;

    use yakui::paint::UserPaintCallId;

    thread_local! {
        static PAINT_CALL_ID_COUNTER: Cell<UserPaintCallId> = const { Cell::new(0) };
        static SHOULD_RERENDER: Cell<bool> = const { Cell::new(true) };
    }

    pub fn reset_paint_state() {
        PAINT_CALL_ID_COUNTER.replace(0);
        SHOULD_RERENDER.set(false);
    }

    pub fn mark_rerender() {
        SHOULD_RERENDER.set(true);
    }

    pub fn should_rerender() -> bool {
        SHOULD_RERENDER.get()
    }

    pub fn new_user_paint_id() -> UserPaintCallId {
        let id = PAINT_CALL_ID_COUNTER.get();

        PAINT_CALL_ID_COUNTER.set(id + 1);

        id
    }
}
