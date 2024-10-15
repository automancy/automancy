use crate::RoundRect;
use crate::ROUNDED_MEDIUM;
use automancy_defs::colors;
use yakui::{
    use_state,
    widgets::{Pad, TextBox, TextBoxResponse},
    Response,
};

fn setup_textbox(mut textbox: TextBox, placeholder: Option<&str>) -> TextBox {
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
pub fn simple_textbox(
    initial_text: &str,
    updated_text: Option<&str>,
    placeholder: Option<&str>,
) -> Response<TextBoxResponse> {
    let first_time = use_state(|| true);

    if first_time.get() {
        first_time.set(false);

        setup_textbox(TextBox::new(Some(initial_text.into())), placeholder).show()
    } else {
        setup_textbox(TextBox::new(updated_text.map(Into::into)), placeholder).show()
    }
}

#[track_caller]
pub fn textbox(
    text: &mut String,
    initial: Option<&str>,
    placeholder: Option<&str>,
) -> Response<TextBoxResponse> {
    let last_text = use_state(|| "".to_string());
    let textbox = use_state(|| None as Option<TextBox>);

    if let Some(textbox) = textbox.borrow_mut().as_mut() {
        textbox.update_text = None
    }

    if textbox.borrow().is_none() {
        textbox.set(Some(setup_textbox(
            TextBox::new(Some(initial.unwrap_or(text).to_string())),
            placeholder,
        )))
    } else if text.as_str() != last_text.borrow().as_str() {
        textbox.set(Some(setup_textbox(
            TextBox::new(Some(text.to_string())),
            placeholder,
        )));
    }

    let mut r = None;

    RoundRect::new(ROUNDED_MEDIUM, colors::ORANGE).show_children(|| {
        Pad::all(2.0).show(|| {
            let res = textbox.borrow().clone().unwrap().show();

            if let Some(updated_text) = &res.text {
                if text.as_str() != updated_text.as_str() {
                    text.clone_from(updated_text);
                }
            }

            r = Some(res);

            last_text.set(text.clone());
        });
    });

    r.unwrap()
}
