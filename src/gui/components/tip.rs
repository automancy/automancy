use std::cell::Cell;

use automancy_defs::colors;
use yakui::{
    constrained,
    widgets::{Layer, Text},
    Constraints, Vec2,
};

use crate::GameState;

use super::{hover::hover_tip, interactive::interactive, symbol, text::label_text};

thread_local! {
    static HOVER_TIP: Cell<Option<Text>> = Cell::default();
}

pub(crate) fn render_info_tip(state: &mut GameState) {
    if let Some(tip) = HOVER_TIP.take() {
        Layer::new().show(|| {
            hover_tip(|| {
                constrained(
                    Constraints::loose(
                        state
                            .gui
                            .as_ref()
                            .unwrap()
                            .yak
                            .layout_dom()
                            .viewport()
                            .size()
                            .min(Vec2::new(500.0, f32::INFINITY)),
                    ),
                    || {
                        tip.show();
                    },
                );
            });
        });
    }
}

pub fn info_tip(info: &str) {
    let label = interactive(|| {
        symbol("\u{f449}", colors::BLACK);
    });

    if label.hovering {
        HOVER_TIP.set(Some(label_text(info)));
    }
}
