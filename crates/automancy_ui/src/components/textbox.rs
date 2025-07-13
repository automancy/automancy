use automancy_data::colors;
use yakui::{
    Response,
    widgets::{Pad, TextBox, TextBoxResponse},
};

use crate::{ROUNDED_MEDIUM, RoundRect};

pub fn setup_textbox(mut textbox: TextBox, placeholder: Option<&str>) -> TextBox {
    if let Some(placeholder) = placeholder {
        textbox.placeholder = placeholder.to_string();
    }

    textbox.radius = ROUNDED_MEDIUM;
    textbox.fill = Some(colors::BACKGROUND_2);
    textbox.selection_halo_color = colors::BACKGROUND_3;
    textbox.selected_bg_color = colors::LIGHT_BLUE;
    textbox.style.color = colors::BLACK;

    textbox
}

#[track_caller]
pub fn simple_textbox(text: &mut String, placeholder: Option<&str>) -> Response<TextBoxResponse> {
    let mut response = setup_textbox(TextBox::new(text.as_str()), placeholder).show();

    if let Some(new_text) = response.text.take() {
        *text = new_text;
    }

    response
}

#[track_caller]
pub fn textbox(text: &mut String, placeholder: Option<&str>) -> Response<TextBoxResponse> {
    let mut response = None;

    RoundRect::new(ROUNDED_MEDIUM, colors::ORANGE).show_children(|| {
        Pad::all(2.0).show(|| {
            response = Some(simple_textbox(text, placeholder));
        });
    });

    response.unwrap()
}
