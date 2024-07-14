use automancy_defs::colors;
use yakui::{
    opaque,
    widgets::{Button, ButtonResponse, DynamicButtonStyle, Pad, Text},
    Color, Response,
};

use crate::gui::ROUNDED_MEDIUM;

use super::{colored_label_text, label_text, symbol_text};

pub fn button_styled(text: Text, padding: Pad) -> Button {
    let mut button = Button::unstyled(text.text);

    let text_style = text.style.clone();

    button.padding = padding;

    button.border_radius = ROUNDED_MEDIUM;

    button.style = DynamicButtonStyle {
        text: text_style.clone(),
        fill: colors::LIGHT_GRAY,
    };

    button.hover_style = DynamicButtonStyle {
        text: text_style.clone(),
        fill: colors::LIGHT_GRAY.adjust(1.2),
    };

    button.down_style = DynamicButtonStyle {
        text: text_style.clone(),
        fill: colors::LIGHT_BLUE.adjust(0.8),
    };

    button
}

pub fn button_text(text: Text) -> Button {
    button_styled(text, Pad::all(8.0))
}

#[track_caller]
pub fn selectable_symbol_button(
    symbol: &str,
    color: Color,
    selected: bool,
) -> Response<ButtonResponse> {
    let mut button = button_styled(symbol_text(symbol, color), Pad::all(2.0));

    if selected {
        button.style.fill = colors::LIGHT_BLUE;
        button.hover_style.fill = colors::LIGHT_BLUE.adjust(1.5);
    }

    button.show()
}

#[track_caller]
pub fn symbol_button(symbol: &str, color: Color) -> Response<ButtonResponse> {
    selectable_symbol_button(symbol, color, false)
}

#[track_caller]
pub fn inactive_button(text: &str) -> Response<ButtonResponse> {
    let mut r = None;

    opaque(|| {
        Pad::all(2.0).show(|| {
            r = Some(button_text(colored_label_text(text, colors::GRAY)).show());
        });
    });

    r.unwrap()
}

#[track_caller]
pub fn button(text: &str) -> Response<ButtonResponse> {
    let mut r = None;

    Pad::all(2.0).show(|| {
        r = Some(button_text(label_text(text)).show());
    });

    r.unwrap()
}
