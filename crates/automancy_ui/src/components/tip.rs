use std::cell::Cell;

use automancy_data::colors;
use yakui::widgets::Text;

use crate::{interactive, label_text, symbol};

thread_local! {
    pub static HOVER_TIP: Cell<Option<Text>> = Cell::default();
}

#[track_caller]
pub fn info_tip(info: &str) {
    let label = interactive(|| {
        symbol("\u{f449}", colors::BLACK);
    });

    if label.hovering {
        HOVER_TIP.set(Some(label_text(info)));
    }
}
