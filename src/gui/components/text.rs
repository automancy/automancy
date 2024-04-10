use automancy_defs::{colors::BLACK, math::Float};
use yakui::{
    pad,
    style::TextStyle,
    util::widget,
    widget::Widget,
    widgets::{Pad, RenderText},
    Color,
};

use super::{HEADING_SIZE, TEXT_SIZE};

use yakui::Response;

#[derive(Debug)]
#[non_exhaustive]
pub struct Text {
    pub text: String,
    pub style: TextStyle,
    pub padding: Pad,
}

impl Text {
    pub fn new(font_size: f32, text: &str) -> Self {
        let mut style = TextStyle::label();
        style.font_size = font_size;

        Self {
            text: text.to_string(),
            style,
            padding: Pad::ZERO,
        }
    }

    pub fn label(text: &str) -> Self {
        Self {
            text: text.to_string(),
            style: TextStyle::label(),
            padding: Pad::all(8.0),
        }
    }

    pub fn show(self) -> Response<TextResponse> {
        widget::<TextWidget>(self)
    }
}

#[derive(Debug)]
pub struct TextWidget {
    props: Text,
}

pub type TextResponse = ();

impl Widget for TextWidget {
    type Props<'a> = Text;
    type Response = TextResponse;

    fn new() -> Self {
        Self {
            props: Text::new(0.0, ""),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        let mut render = RenderText::label(self.props.text.clone().into());
        render.style = self.props.style.clone();

        pad(self.props.padding, || {
            render.show();
        });
    }
}

pub fn sized_colored_label(text: &str, font_size: Float, color: Color) -> Response<TextResponse> {
    let mut text = Text::label(text);
    text.style.color = color;
    text.style.font_size = font_size;
    text.show()
}

pub fn colored_label(text: &str, color: Color) -> Response<TextResponse> {
    sized_colored_label(text, TEXT_SIZE, color)
}

pub fn label(text: &str) -> Response<TextResponse> {
    sized_colored_label(text, TEXT_SIZE, BLACK)
}

pub fn heading(text: &str) -> Response<TextResponse> {
    sized_colored_label(text, HEADING_SIZE, BLACK)
}
