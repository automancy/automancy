use automancy_defs::{colors::BLACK, math::Float};
use yakui::{
    font::FontName,
    pad,
    style::TextStyle,
    util::widget,
    widget::Widget,
    widgets::{Pad, RenderText},
    Color,
};

use super::{HEADING_SIZE, LABEL_SIZE, PADDING_MEDIUM};

use yakui::Response;

#[derive(Debug, Clone)]
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

    fn label(text: &str) -> Self {
        Self {
            text: text.to_string(),
            style: TextStyle::label(),
            padding: Pad::all(PADDING_MEDIUM),
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

pub fn sized_colored_text(text: &str, font_size: Float, font: FontName, color: Color) -> Text {
    let mut text = Text::label(text);
    text.style.color = color;
    text.style.font_size = font_size;
    text.style.font = font;
    text
}

pub fn colored_label_text(text: &str, color: Color) -> Text {
    sized_colored_text(text, LABEL_SIZE, "default".into(), color)
}

pub fn colored_label(text: &str, color: Color) -> Response<TextResponse> {
    colored_label_text(text, color).show()
}

pub fn label_text(text: &str) -> Text {
    sized_colored_text(text, LABEL_SIZE, "default".into(), BLACK)
}

pub fn symbol_text(symbol: &str, color: Color) -> Text {
    sized_colored_text(symbol, LABEL_SIZE, "symbols".into(), color)
}

pub fn label(text: &str) -> Response<TextResponse> {
    label_text(text).show()
}

pub fn heading(text: &str) -> Response<TextResponse> {
    sized_colored_text(text, HEADING_SIZE, "default".into(), BLACK).show()
}
