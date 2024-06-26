use automancy_defs::colors;
use yakui::{
    align,
    geometry::Color,
    opaque,
    style::{TextAlignment, TextStyle},
    util::widget,
};
use yakui::{
    event::{EventInterest, EventResponse, WidgetEvent},
    widgets::RoundRect,
};
use yakui::{input::MouseButton, pad};
use yakui::{
    widget::{EventContext, Widget},
    widgets::Pad,
};
use yakui::{Alignment, Response};

use super::{
    colored_label_text, symbol_text,
    text::{label_text, Text},
};

/**
A button containing some text.

Responds with [ButtonResponse].

Shorthand:
```rust
# let _handle = yakui_widgets::DocTest::start();
if yakui::button("Hello").clicked {
    println!("The button was clicked");
}
```
*/
#[derive(Debug)]
#[non_exhaustive]
pub struct Button {
    pub text: Text,
    pub padding: Pad,
    pub border_radius: f32,
    pub style: DynamicButtonStyle,
    pub hover_style: DynamicButtonStyle,
    pub down_style: DynamicButtonStyle,
}

/// Contains styles that can vary based on the state of the button.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DynamicButtonStyle {
    pub text: TextStyle,
    pub fill: Color,
}

impl Default for DynamicButtonStyle {
    fn default() -> Self {
        let mut text = TextStyle::label();
        text.align = TextAlignment::Center;

        Self {
            text,
            fill: Color::GRAY,
        }
    }
}

impl Button {
    pub fn unstyled(text: &str) -> Self {
        Self {
            text: label_text(text),
            padding: Pad::ZERO,
            border_radius: 0.0,
            style: DynamicButtonStyle::default(),
            hover_style: DynamicButtonStyle::default(),
            down_style: DynamicButtonStyle::default(),
        }
    }

    pub fn styled(text: Text) -> Self {
        let mut text_style = text.style.clone();
        text_style.align = TextAlignment::Center;

        let style = DynamicButtonStyle {
            text: text_style.clone(),
            fill: colors::LIGHT_GRAY,
            ..Default::default()
        };

        let hover_style = DynamicButtonStyle {
            text: text_style.clone(),
            fill: colors::LIGHT_GRAY.adjust(1.2),
            ..Default::default()
        };

        let down_style = DynamicButtonStyle {
            text: text_style.clone(),
            fill: colors::LIGHT_BLUE.adjust(0.8),
            ..Default::default()
        };

        Self {
            text,
            padding: Pad::all(0.0),
            border_radius: 8.0,
            style,
            hover_style,
            down_style,
        }
    }

    pub fn show(self) -> Response<ButtonResponse> {
        widget::<ButtonWidget>(self)
    }
}

#[derive(Debug)]
pub struct ButtonWidget {
    props: Button,
    hovering: bool,
    mouse_down: bool,
    clicked: bool,
}

#[derive(Debug)]
pub struct ButtonResponse {
    pub hovering: bool,
    pub clicked: bool,
}

impl Widget for ButtonWidget {
    type Props<'a> = Button;
    type Response = ButtonResponse;

    fn new() -> Self {
        Self {
            props: Button::unstyled(""),
            hovering: false,
            mouse_down: false,
            clicked: false,
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        let mut color = self.props.style.fill;
        let mut text_style = self.props.style.text.clone();

        if self.mouse_down {
            let style = &self.props.down_style;
            color = style.fill;
            text_style = style.text.clone();
        } else if self.hovering {
            let style = &self.props.hover_style;
            color = style.fill;
            text_style = style.text.clone();
        }

        let alignment = match text_style.align {
            TextAlignment::Start => Alignment::CENTER_LEFT,
            TextAlignment::Center => Alignment::CENTER,
            TextAlignment::End => Alignment::CENTER_RIGHT,
        };

        let mut container = RoundRect::new(self.props.border_radius);
        container.color = color;
        container.show_children(|| {
            pad(self.props.padding, || {
                align(alignment, || {
                    let mut text = self.props.text.clone();
                    text.style = text_style;
                    text.show();
                });
            });
        });

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

pub fn button_text(text: Text) -> Response<ButtonResponse> {
    let mut r = None;

    Pad::all(2.0).show(|| {
        r = Some(Button::styled(text).show());
    });

    r.unwrap()
}

pub fn selectable_symbol_button(
    symbol: &str,
    color: Color,
    selected: bool,
) -> Response<ButtonResponse> {
    let mut r = None;

    Pad::all(2.0).show(|| {
        let mut button = Button::styled(symbol_text(symbol, color));

        if selected {
            button.style.fill = colors::LIGHT_BLUE;
            button.hover_style.fill = colors::LIGHT_BLUE.adjust(1.5);
        }

        r = Some(button.show());
    });

    r.unwrap()
}

pub fn symbol_button(symbol: &str, color: Color) -> Response<ButtonResponse> {
    selectable_symbol_button(symbol, color, false)
}

pub fn button(text: &str) -> Response<ButtonResponse> {
    button_text(colored_label_text(text, colors::BLACK))
}

pub fn inactive_button(text: &str) -> Response<ButtonResponse> {
    let mut r = None;

    opaque(|| {
        r = Some(button_text(colored_label_text(text, colors::GRAY)));
    });

    r.unwrap()
}
