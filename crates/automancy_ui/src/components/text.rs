use automancy_data::colors::BLACK;
use cosmic_text::FamilyOwned;
use yakui::{
    Alignment, Color, Constraints, Response, Vec2, align, constrained,
    style::TextStyle,
    widgets::{Pad, Text, TextResponse},
};

use crate::{HEADING_SIZE, LABEL_SIZE, PADDING_MEDIUM, SMALL_SIZE, SYMBOLS_FONT_KEY};

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

#[track_caller]
pub fn colored_label(text: &str, color: Color) -> Response<TextResponse> {
    colored_label_text(text, color).show()
}

pub fn sized_text(text: &str, font_size: f32) -> Text {
    colored_sized_text(text, BLACK, font_size)
}

#[track_caller]
pub fn sized(text: &str, font_size: f32) -> Response<TextResponse> {
    sized_text(text, font_size).show()
}

pub fn small_text(text: &str) -> Text {
    sized_text(text, SMALL_SIZE)
}

#[track_caller]
pub fn small(text: &str) -> Response<TextResponse> {
    small_text(text).show()
}

pub fn label_text(text: &str) -> Text {
    colored_sized_text(text, BLACK, LABEL_SIZE)
}

#[track_caller]
pub fn label(text: &str) -> Response<TextResponse> {
    label_text(text).show()
}

pub fn heading_text(text: &str) -> Text {
    sized_text(text, HEADING_SIZE)
}

#[track_caller]
pub fn heading(text: &str) -> Response<TextResponse> {
    heading_text(text).show()
}

pub fn symbol_text(symbol: &str, color: Color) -> Text {
    let mut text = colored_label_text(symbol, color);
    text.style.attrs.family_owned = FamilyOwned::Name(SYMBOLS_FONT_KEY.into());
    text.padding = Pad::ZERO;
    text
}

pub fn symbol(symbol: &str, color: Color) {
    constrained(
        Constraints::tight(Vec2::new(
            LABEL_SIZE + PADDING_MEDIUM,
            LABEL_SIZE + PADDING_MEDIUM,
        )),
        || {
            align(Alignment::CENTER, || {
                symbol_text(symbol, color).show();
            });
        },
    );
}
