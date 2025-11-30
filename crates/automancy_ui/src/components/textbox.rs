use crate::*;

#[track_caller]
pub fn textbox(
    text: &mut String,
    style: TextStyle,
    placeholder: impl Into<Option<Cow<'static, str>>>,
    placeholder_style: Option<TextStyle>,
) -> Response<TextBoxResponse> {
    let mut res = None;
    Pad::all(sizing::PADDING_XSMALL).show(|| {
        res = Some(
            TextBox::normal()
                .placeholder_style(placeholder_style.unwrap_or_else(|| style.clone().color(colors::TEXT_INACTIVE.yak())))
                .placeholder(placeholder.into().unwrap_or_default())
                .style(style)
                .show(text.as_str()),
        );
    });
    let mut res = res.unwrap();

    if let Some(new_text) = res.text.take() {
        *text = new_text;
    }

    res
}

pub trait TextBoxExt {
    fn normal() -> TextBox;
}

impl TextBoxExt for TextBox {
    fn normal() -> TextBox {
        TextBox::new()
            .radius(sizing::ROUNDED_MEDIUM)
            .fill(colors::BACKGROUND_2.yak())
            .selection_halo_color(colors::INTERACTIVE_2.yak())
            .selected_bg_color(colors::TEXT_BG_SELECTED.yak())
            .cursor_color(colors::IMPORTANT.yak())
    }
}
