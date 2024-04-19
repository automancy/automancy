use automancy_defs::colors;
use yakui::{
    util::widget_children,
    widget::{LayoutContext, Widget},
    widgets::{Absolute, Layer},
    Alignment, Constraints, Pivot, Rect, Response, Vec2,
};

use crate::gui::util::constrain_to_viewport;

use super::container::RoundRect;

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
            let mut rect = Rect::from_pos_size(pos, size);
            constrain_to_viewport(&mut rect, ctx.layout);

            for &child in &node.children {
                ctx.layout.set_pos(child, rect.pos());
            }
        }

        size
    }
}

pub fn follow_cursor(children: impl FnOnce()) {
    Absolute::new(Alignment::TOP_LEFT, Pivot::TOP_LEFT, Vec2::ZERO).show(|| {
        Hover::new().show(|| {
            Layer::new().show(children);
        });
    });
}

pub fn hover_tip(children: impl FnOnce()) {
    follow_cursor(|| {
        RoundRect::new(8.0, colors::BACKGROUND_1).show_children(|| {
            children();
        });
    });
}
