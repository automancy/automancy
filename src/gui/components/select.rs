use yakui::{column, use_state, Alignment, Dim2, Pivot};

use super::{button::button, layer::Layer, relative::Relative, scrollable::scroll_vertical};

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
                Relative::new(Alignment::BOTTOM_LEFT, Pivot::TOP_LEFT, Dim2::ZERO).show(|| {
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
        }
    });

    selected
}
