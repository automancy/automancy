use automancy_defs::colors::BLACK;
use yakui::{
    constrained, pad,
    style::TextStyle,
    util::widget,
    widget::Widget,
    widgets::{Pad, RenderText},
    Color, Constraints, Vec2,
};

use super::{
    centered_column, centered_row, HEADING_SIZE, LABEL_SIZE, PADDING_MEDIUM, PADDING_SMALL,
    SMALL_SIZE,
};

use yakui::Response;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Text {
    pub text: String,
    pub style: TextStyle,
    pub padding: Pad,
    pub square_sized: bool,
}

impl Text {
    pub fn new(font_size: f32, color: Color, text: &str, square_sized: bool) -> Self {
        let mut style = TextStyle::label();
        style.font_size = font_size;
        style.color = color;

        Self {
            text: text.to_string(),
            style,
            padding: Pad::ZERO,
            square_sized,
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
            props: Text::new(0.0, BLACK, "", false),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        let mut render = RenderText::label(self.props.text.clone().into());
        render.style = self.props.style.clone();

        pad(self.props.padding, || {
            if self.props.square_sized {
                constrained(
                    Constraints::tight(Vec2::new(LABEL_SIZE, LABEL_SIZE)),
                    || {
                        centered_column(|| {
                            centered_row(|| {
                                render.show();
                            });
                        });
                    },
                );
            } else {
                render.show();
            }
        });
    }
}

pub fn colored_label_text(text: &str, color: Color) -> Text {
    let mut text = Text::new(LABEL_SIZE, color, text, false);
    text.padding = Pad::all(PADDING_MEDIUM);
    text
}

pub fn colored_label(text: &str, color: Color) -> Response<TextResponse> {
    colored_label_text(text, color).show()
}

pub fn label_text(text: &str) -> Text {
    colored_label_text(text, BLACK)
}

pub fn small_text(text: &str) -> Text {
    let mut text = label_text(text);
    text.style.font_size = SMALL_SIZE;
    text
}

pub fn headin_text(text: &str) -> Text {
    let mut text = label_text(text);
    text.style.font_size = HEADING_SIZE;
    text
}

pub fn symbol_text(symbol: &str, color: Color) -> Text {
    let mut text = Text::new(LABEL_SIZE, color, symbol, true);
    text.padding = Pad::all(PADDING_SMALL);
    text.style.font = "symbols".into();
    text
}

pub fn label(text: &str) -> Response<TextResponse> {
    label_text(text).show()
}

pub fn heading(text: &str) -> Response<TextResponse> {
    headin_text(text).show()
}

pub fn symbol(symbol: &str, color: Color) {
    symbol_text(symbol, color).show();
}
