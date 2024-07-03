use automancy_defs::colors;
use yakui::{
    use_state,
    widgets::{Pad, TextBox, TextBoxResponse},
    Response,
};

use crate::gui::ROUNDED_MEDIUM;

use super::RoundRect;

fn set_textbox(textbox: &mut TextBox, placeholder: Option<&str>) {
    if let Some(placeholder) = placeholder {
        textbox.placeholder = placeholder.to_string();
    }

    textbox.radius = ROUNDED_MEDIUM;
    textbox.fill = Some(colors::BACKGROUND_2);
    textbox.selection_halo_color = colors::WHITE;
    textbox.selected_bg_color = colors::LIGHT_BLUE;
    textbox.style.color = colors::BLACK;
}

pub fn textbox(
    text: &mut String,
    initial: Option<&str>,
    placeholder: Option<&str>,
) -> Response<TextBoxResponse> {
    let last_text = use_state(|| None as Option<String>);

    let mut r = None;

    RoundRect::new(ROUNDED_MEDIUM, colors::ORANGE).show_children(|| {
        Pad::all(2.0).show(|| {
            if last_text.borrow().is_some()
                && text.as_str() != last_text.borrow().as_ref().unwrap().as_str()
            {
                let mut textbox = TextBox::with_text(initial.unwrap_or(text), Some(text));
                set_textbox(&mut textbox, placeholder);

                let res = textbox.show();
                if let Some(new_text) = &res.text {
                    text.clone_from(new_text);
                }

                r = Some(res);
            } else {
                let mut textbox = TextBox::with_text(initial.unwrap_or(text), None);
                set_textbox(&mut textbox, placeholder);

                let res = textbox.show();
                if let Some(new_text) = &res.text {
                    text.clone_from(new_text);
                }

                r = Some(res);
            }

            last_text.set(Some(text.clone()));
        });
    });

    r.unwrap()
}
