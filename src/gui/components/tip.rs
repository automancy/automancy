use std::cell::Cell;

use automancy_defs::colors;
use yakui::{constrained, widgets::Layer, Constraints, Vec2};

use crate::GameState;

use super::{
    hover::hover_tip,
    interactive::interactive,
    text::{label_text, symbol_text, Text},
};

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
                            .yak
                            .layout_dom()
                            .viewport()
                            .size()
                            .min(Vec2::new(400.0, f32::INFINITY)),
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
        symbol_text("\u{f449}", colors::BLACK).show();
    });

    if label.hovering {
        HOVER_TIP.set(Some(label_text(info)));
    }
}
