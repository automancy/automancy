use yakui::{
    util::widget_children,
    widget::{LayoutContext, Widget},
    widgets::Layer,
    Alignment, Constraints, Pivot, Response, Vec2,
};

use super::absolute::Absolute;

#[derive(Debug, Default)]
pub struct Hover {}

impl Hover {
    fn new() -> Self {
        Self::default()
    }

    fn show<F: FnOnce()>(self, children: F) -> Response<HoverResponse> {
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

    fn layout(&self, mut ctx: LayoutContext<'_>, _constraints: Constraints) -> Vec2 {
        let node = ctx.dom.get_current();

        let mut size = Vec2::ZERO;
        for &child in &node.children {
            size = size.max(ctx.calculate_layout(child, Constraints::none()));
        }

        if let Some(pos) = ctx.input.get_mouse_position(ctx.layout) {
            for &child in &node.children {
                ctx.layout.set_pos(child, pos);
            }
        }

        Vec2::ZERO
    }
}

pub fn follow_cursor(children: impl FnOnce()) {
    Absolute::new(Alignment::TOP_LEFT, Pivot::TOP_LEFT, Vec2::ZERO).show(|| {
        children();
    });
}
