use crate::*;

#[derive(Debug, Clone, Copy, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct Interactive {}

impl Interactive {
    pub fn new() -> Self {
        Self::default()
    }

    #[track_caller]
    pub fn show<F: FnOnce()>(self, children: F) -> Response<InteractiveResponse> {
        widget_children::<InteractiveWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct InteractiveWidget {
    props: Interactive,
    hovering: bool,
    mouse_down: bool,
    clicked: bool,
}

#[derive(Debug)]
pub struct InteractiveResponse {
    pub hovering: bool,
    pub clicked: bool,
}

impl Widget for InteractiveWidget {
    type Props<'a> = Interactive;
    type Response = InteractiveResponse;

    fn new() -> Self {
        Self {
            props: Interactive::new(),
            hovering: false,
            mouse_down: false,
            clicked: false,
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        let clicked = self.clicked;
        self.clicked = false;

        Self::Response {
            hovering: self.hovering,
            clicked,
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
            &WidgetEvent::MouseButtonChanged {
                button: MouseButton::One,
                down,
                inside,
                ..
            } => {
                let prev_down = self.mouse_down;
                self.mouse_down = down;

                if inside && down && !prev_down {
                    self.clicked = true;
                    return EventResponse::Sink;
                }

                EventResponse::Bubble
            },
            _ => EventResponse::Bubble,
        }
    }
}

#[track_caller]
pub fn interactive(children: impl FnOnce()) -> Response<InteractiveResponse> {
    Interactive::new().show(children)
}
