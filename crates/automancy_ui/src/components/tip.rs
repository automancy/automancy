use crate::{interactive, label_text, symbol};
use automancy_defs::colors;
use std::cell::Cell;
use yakui::widgets::Text;

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
