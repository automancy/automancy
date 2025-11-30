use crate::*;

pub trait ButtonExt {
    fn with_text(text: Text) -> Button;

    fn simple(text: Text) -> Button;
}

impl ButtonExt for Button {
    fn with_text(text: Text) -> Button {
        let text_style = text.style.clone().align(TextAlignment::Center);

        Button::unstyled(text.text)
            .border_radius(sizing::ROUNDED_MEDIUM)
            .style(DynamicButtonStyle {
                text: text_style.clone(),
                fill: colors::BACKGROUND_2.yak(),
                border: None,
            })
            .hover_style(DynamicButtonStyle {
                text: text_style.clone(),
                fill: colors::BUTTON_HOVERED.yak(),
                border: None,
            })
            .down_style(DynamicButtonStyle {
                text: text_style.clone(),
                fill: colors::BUTTON_PRESSED.yak(),
                border: None,
            })
    }

    fn simple(text: Text) -> Button {
        Self::with_text(text).padding(Pad::all(sizing::PADDING_MEDIUM))
    }
}

#[track_caller]
pub fn selectable_symbol_button<S: Into<Cow<'static, str>>>(symbol: S, color: Color, selected: bool) -> Response<ButtonResponse> {
    let mut button = Button::simple(Text::symbol(symbol).color(color)).padding(Pad::all(sizing::PADDING_XSMALL));

    if selected {
        button.style.fill = colors::BUTTON_HOVERED.yak();
        button.hover_style.fill = colors::BUTTON_PRESSED.yak();
    }

    button.show()
}

#[track_caller]
pub fn symbol_button<S: Into<Cow<'static, str>>>(symbol: S, color: Color) -> Response<ButtonResponse> {
    selectable_symbol_button(symbol, color, false)
}

#[track_caller]
pub fn inactive_button<S: Into<Cow<'static, str>>>(text: S) -> Response<ButtonResponse> {
    let mut response = None;

    Opaque::new().show(|| {
        Pad::all(sizing::PADDING_XSMALL).show(|| {
            response = Some(Button::simple(Text::normal(text).color(colors::BACKGROUND_INACTIVE.yak())).show());
        });
    });

    let mut response = response.unwrap();
    response.clicked = false;
    response
}

#[track_caller]
pub fn button<S: Into<Cow<'static, str>>>(text: S) -> Response<ButtonResponse> {
    let mut response = None;

    Pad::all(sizing::PADDING_XSMALL).show(|| {
        response = Some(Button::simple(Text::normal(text)).show());
    });

    response.unwrap()
}
