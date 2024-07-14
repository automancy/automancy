use yakui::{
    event::{EventInterest, EventResponse, WidgetEvent},
    input::MouseButton,
    util::widget_children,
    widget::{EventContext, Widget},
    Response,
};

#[derive(Debug, Default)]
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
            }
            WidgetEvent::MouseLeave => {
                self.hovering = false;
                EventResponse::Sink
            }
            WidgetEvent::MouseButtonChanged {
                button: MouseButton::One,
                down,
                inside,
                ..
            } => {
                if *inside {
                    if *down {
                        self.mouse_down = true;
                        EventResponse::Sink
                    } else if self.mouse_down {
                        self.mouse_down = false;
                        self.clicked = true;
                        EventResponse::Sink
                    } else {
                        EventResponse::Bubble
                    }
                } else {
                    if !*down {
                        self.mouse_down = false;
                    }

                    EventResponse::Bubble
                }
            }
            _ => EventResponse::Bubble,
        }
    }
}

#[track_caller]
pub fn interactive(children: impl FnOnce()) -> Response<InteractiveResponse> {
    Interactive::new().show(children)
}
