use std::cell::Cell;

use automancy_data::colors;
use yakui::{
    Alignment, Constraints, Dim2, Flow, Response, Vec2,
    event::{EventInterest, EventResponse, WidgetEvent},
    layout::{AbstractClipRect, ClipLogic},
    util::widget_children,
    widget::{EventContext, LayoutContext, Widget},
};

use crate::RoundRect;

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
    mouse_pos: Cell<Option<Vec2>>,
}

pub type HoverResponse = ();

impl Widget for HoverWidget {
    type Props<'a> = Hover;
    type Response = HoverResponse;

    fn new() -> Self {
        Self {
            props: Hover::new(),
            pos: Cell::default(),
            mouse_pos: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn flow(&self) -> Flow {
        Flow::Relative {
            anchor: Alignment::new(self.pos.get().x, self.pos.get().y),
            offset: Dim2::ZERO,
        }
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_ALL
    }

    fn event(&mut self, _ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        if let WidgetEvent::MouseMoved(v) = event {
            self.mouse_pos.set(*v)
        }

        EventResponse::Bubble
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, _constraints: Constraints) -> Vec2 {
        ctx.layout.set_clip_logic(
            ctx.dom,
            ClipLogic::Contain {
                it: AbstractClipRect::LayoutRect,
                parent: AbstractClipRect::ParentClip,
            },
        );

        let node = ctx.dom.get_current();

        let mut size = Vec2::ZERO;
        for &child in &node.children {
            size = size.max(ctx.calculate_layout(child, Constraints::none()));
        }

        if let Some(pos) = self.mouse_pos.get() {
            let pos = pos + Vec2::new(10.0, 0.0);
            self.pos.set(pos);
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
