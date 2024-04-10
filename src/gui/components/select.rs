use yakui::{
    align, reflow,
    widgets::{Layer, Pad},
    Alignment, Dim2, Pivot,
};

use super::{button::button, list::column};

pub fn selection_box<T: Clone + Eq>(
    options: impl IntoIterator<Item = T>,
    default: T,
    format: &dyn Fn(T) -> String,
) -> T {
    let mut open = false;
    let mut selected = default;

    align(Alignment::TOP_LEFT, || {
        column(|| {
            if button(&format(selected.clone())).clicked {
                open = !open;
            }

            if open {
                Pad::ZERO.show(|| {
                    Layer::new().show(|| {
                        reflow(Alignment::BOTTOM_LEFT, Pivot::TOP_LEFT, Dim2::ZERO, || {
                            column(|| {
                                for option in options.into_iter() {
                                    if option != selected && button(&format(option.clone())).clicked
                                    {
                                        selected = option;
                                        open = false;
                                    }
                                }
                            });
                        });
                    });
                });
            }
        });
    });

    selected
}
