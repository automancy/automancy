use crate::clamp_percentage_to_viewport;
use crate::RoundRect;
use automancy_defs::colors;
use std::cell::Cell;
use yakui::{
    util::widget_children,
    widget::{LayoutContext, Widget},
    Alignment, Constraints, Dim2, Flow, Response, Vec2,
};

#[derive(Debug, Default)]
pub struct Hover {}

impl Hover {
    pub fn new() -> Self {
        Self::default()
    }

    #[track_caller]
    pub fn show<F: FnOnce()>(self, children: F) -> Response<HoverResponse> {
        widget_children::<HoverWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct HoverWidget {
    props: Hover,
    pos: Cell<Vec2>,
}

pub type HoverResponse = ();

impl Widget for HoverWidget {
    type Props<'a> = Hover;
    type Response = HoverResponse;

    fn new() -> Self {
        Self {
            props: Hover::new(),
            pos: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn flow(&self) -> Flow {
        Flow::Absolute {
            anchor: Alignment::new(self.pos.get().x, self.pos.get().y),
            offset: Dim2::ZERO,
        }
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, _constraints: Constraints) -> Vec2 {
        let node = ctx.dom.get_current();

        let mut size = Vec2::ZERO;
        for &child in &node.children {
            size = size.max(ctx.calculate_layout(child, Constraints::none()));
        }

        if let Some(pos) = ctx.input.get_mouse_position(ctx.layout) {
            let pos = pos + Vec2::new(10.0, 0.0);
            self.pos.set(clamp_percentage_to_viewport(
                size,
                pos / ctx.layout.viewport().size(),
                ctx.layout.viewport(),
            ));
        }

        size
    }
}

#[track_caller]
pub fn hover_tip(children: impl FnOnce()) {
    Hover::new().show(|| {
        RoundRect::new(8.0, colors::BACKGROUND_1).show_children(|| {
            children();
        });
    });
}
