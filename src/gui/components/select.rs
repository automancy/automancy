use automancy_defs::colors;
use yakui::{column, use_state, widgets::Pad, Alignment, Dim2, Pivot};

use super::{
    button::button, container::RoundRect, layer::Layer, relative::Relative,
    scrollable::scroll_vertical, PADDING_MEDIUM,
};

pub fn selection_box<T: Clone + Eq>(
    options: impl IntoIterator<Item = T>,
    default: T,
    format: &dyn Fn(&T) -> String,
) -> T {
    let open = use_state(|| false);
    let mut selected = default;

    column(|| {
        if button(&format(&selected)).clicked {
            open.modify(|v| !v);
        }

        if open.get() {
            Layer::new().show(|| {
                Relative::new(Alignment::TOP_LEFT, Pivot::TOP_LEFT, Dim2::ZERO).show(|| {
                    let mut container = RoundRect::new(8.0);
                    container.color = colors::BACKGROUND_1;
                    container.show_children(|| {
                        Pad::all(PADDING_MEDIUM).show(|| {
                            scroll_vertical(250.0, || {
                                column(|| {
                                    for option in options.into_iter() {
                                        if button(&format(&option)).clicked {
                                            selected = option;
                                            open.set(false);
                                        }
                                    }
                                });
                            });
                        });
                    });
                });
            });
        }
    });

    selected
}
