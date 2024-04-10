use core::fmt;

use yakui::{
    util::widget_children,
    widget::{LayoutContext, Widget},
    Constraints, Response, Vec2,
};

pub struct Hover {}

impl Hover {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show<F: FnOnce()>(self, children: F) -> Response<HoverResponse> {
        widget_children::<HoverWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct HoverWidget {
    props: Hover,
}

pub type HoverResponse = ();

impl Widget for HoverWidget {
    type Props<'a> = Hover;
    type Response = HoverResponse;

    fn new() -> Self {
        Self {
            props: Hover::new(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, ctx: LayoutContext<'_>, _constraints: Constraints) -> Vec2 {
        let id = ctx.dom.current();

        if let Some(pos) = ctx.input.get_mouse_position() {
            ctx.layout.set_pos(id, pos);
        }

        Vec2::ZERO
    }
}

impl fmt::Debug for Hover {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hover").finish_non_exhaustive()
    }
}
