use crate::*;

#[derive(Debug, Clone, Copy, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
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
    mouse_pos: Option<Vec2>,
}

pub type HoverResponse = ();

impl Widget for HoverWidget {
    type Props<'a> = Hover;
    type Response = HoverResponse;

    fn new() -> Self {
        Self {
            props: Hover::new(),
            mouse_pos: None,
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn flow(&self) -> Flow {
        Flow::Relative {
            anchor: Alignment::TOP_LEFT,
            offset: Dim2::ZERO,
        }
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, _constraints: Constraints) -> Vec2 {
        let node = ctx.dom.get_current();

        let Some(pos) = self.mouse_pos else {
            return Vec2::ZERO;
        };

        let mut size = Vec2::ZERO;
        for &child in &node.children {
            size = size.max(ctx.calculate_layout(child, Constraints::loose(ctx.layout.viewport().size())));
        }

        let pos = pos + Vec2::new(0.0, -size.y - 8.0);

        ctx.layout.set_clip_logic(
            ctx.dom,
            ClipLogic::Contain {
                it: AbstractClipRect::Value(Rect::from_pos_size(Vec2::ZERO, size)),
                parent: AbstractClipRect::Viewport,
                offset: pos,
            },
        );

        Vec2::ZERO
    }

    fn paint(&self, ctx: PaintContext<'_>) {
        if self.mouse_pos.is_some() {
            self.default_paint(ctx);
        }
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_ALL
    }

    fn event(&mut self, _ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        if let WidgetEvent::MouseMoved(v) = event {
            self.mouse_pos = *v
        }

        EventResponse::Bubble
    }
}

pub fn hover_tip(children: impl FnOnce()) {
    Hover::new().show(|| {
        RoundRect::new(sizing::ROUNDED_MEDIUM)
            .color(colors::BACKGROUND_OPAQUE.yak())
            .show_children(|| {
                Pad::all(sizing::PADDING_SMALL).show(|| {
                    children();
                });
            });
    });
}
