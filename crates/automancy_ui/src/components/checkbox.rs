use crate::*;

const OUTER_SIZE: Vec2 = Vec2::splat(24.0);
const INNER_SIZE: Vec2 = Vec2::splat(16.0);

#[derive(Debug, Clone, Copy)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct Checkbox {
    pub checked: bool,
}

impl Checkbox {
    pub fn new(checked: bool) -> Self {
        Self {
            checked,
        }
    }

    #[track_caller]
    pub fn show(self) -> Response<CheckboxResponse> {
        widget::<CheckboxWidget>(self)
    }
}

#[derive(Debug)]
pub struct CheckboxWidget {
    props: Checkbox,
    hovering: bool,
    mouse_down: bool,
    just_toggled: bool,
}

#[derive(Debug)]
pub struct CheckboxResponse {
    pub checked: bool,
}

impl Widget for CheckboxWidget {
    type Props<'a> = Checkbox;
    type Response = CheckboxResponse;

    fn new() -> Self {
        Self {
            props: Checkbox::new(false),
            hovering: false,
            mouse_down: false,
            just_toggled: false,
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        let mut checked = self.props.checked;
        if self.just_toggled {
            checked = !checked;
            self.just_toggled = false;
        }

        CheckboxResponse {
            checked,
        }
    }

    fn layout(&self, _ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        constraints.constrain_min(OUTER_SIZE)
    }

    fn paint(&self, ctx: PaintContext<'_>) {
        let layout_node = ctx.layout.get(ctx.dom.current()).unwrap();

        let padding = OUTER_SIZE - INNER_SIZE;
        let mut check_rect = layout_node.rect;
        check_rect.set_pos(check_rect.pos() + padding * 0.5);
        check_rect.set_size(check_rect.size() - padding);

        shapes::PaintRoundRect::new(layout_node.rect, 6.0)
            .color(colors::BACKGROUND_2.yak())
            .add(ctx.paint);

        if self.props.checked {
            shapes::cross(ctx.paint, check_rect, colors::INTERACTIVE_2.yak());
        }
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_INSIDE | EventInterest::MOUSE_OUTSIDE
    }

    fn event(&mut self, _ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        match event {
            WidgetEvent::MouseEnter => {
                self.hovering = true;
                EventResponse::Sink
            },
            WidgetEvent::MouseLeave => {
                self.hovering = false;
                EventResponse::Sink
            },
            WidgetEvent::MouseButtonChanged {
                button: MouseButton::One,
                down,
                inside,
                ..
            } => {
                if *inside {
                    if *down {
                        self.mouse_down = true;
                        return EventResponse::Sink;
                    } else if self.mouse_down {
                        self.mouse_down = false;
                        self.just_toggled = true;
                    }
                } else {
                    self.mouse_down = false;
                }

                EventResponse::Bubble
            },
            _ => EventResponse::Bubble,
        }
    }
}

#[track_caller]
pub fn checkbox(v: &mut bool) {
    Pad::all(sizing::PADDING_SMALL).show(|| {
        *v = Checkbox::new(*v).show().checked;
    });
}
