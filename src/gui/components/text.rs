use automancy_defs::colors::BLACK;
use yakui::{
    style::TextStyle,
    widgets::{ConstrainedBox, Pad, Text, TextResponse},
    Color, Constraints, Response, Vec2,
};

use crate::SYMBOLS_FONT_KEY;

use super::{
    centered_column, centered_row, HEADING_SIZE, LABEL_SIZE, PADDING_MEDIUM, PADDING_SMALL,
    SMALL_SIZE,
};

pub fn colored_sized_text(text: &str, color: Color, font_size: f32) -> Text {
    let mut text = Text::with_style(
        text.to_owned(),
        TextStyle {
            font_size,
            color,
            ..Default::default()
        },
    );
    text.padding = Pad::all(PADDING_MEDIUM);
    text
}

pub fn colored_label_text(text: &str, color: Color) -> Text {
    colored_sized_text(text, color, LABEL_SIZE)
}

pub fn colored_label(text: &str, color: Color) -> Response<TextResponse> {
    colored_label_text(text, color).show()
}

pub fn sized_text(text: &str, font_size: f32) -> Text {
    colored_sized_text(text, BLACK, font_size)
}

pub fn sized(text: &str, font_size: f32) -> Response<TextResponse> {
    sized_text(text, font_size).show()
}

pub fn small_text(text: &str) -> Text {
    sized_text(text, SMALL_SIZE)
}

pub fn small(text: &str) -> Response<TextResponse> {
    small_text(text).show()
}

pub fn label_text(text: &str) -> Text {
    colored_sized_text(text, BLACK, LABEL_SIZE)
}

pub fn label(text: &str) -> Response<TextResponse> {
    label_text(text).show()
}

pub fn heading_text(text: &str) -> Text {
    sized_text(text, HEADING_SIZE)
}

pub fn heading(text: &str) -> Response<TextResponse> {
    heading_text(text).show()
}

pub fn symbol_text(symbol: &str, color: Color) -> Text {
    let mut text = colored_label_text(symbol, color);
    text.style.attrs.family_owned = cosmic_text::FamilyOwned::Name(SYMBOLS_FONT_KEY.to_owned());
    text.padding = Pad::all(PADDING_SMALL);
    text
}

pub fn symbol(symbol: &str, color: Color) {
    ConstrainedBox::new(Constraints::tight(Vec2::new(
        LABEL_SIZE + PADDING_SMALL * 2.0,
        LABEL_SIZE + PADDING_SMALL * 2.0,
    )))
    .show(|| {
        centered_row(|| {
            centered_column(|| {
                symbol_text(symbol, color).show();
            });
        });
    });
}
