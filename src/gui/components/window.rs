use automancy_defs::colors;
use yakui::{colored_box_container, row, widgets::Pad};

use super::{layer::Layer, layout::centered_column, text::heading, PADDING_LARGE, PADDING_MEDIUM};

pub fn window(title: String, children: impl FnOnce()) {
    Layer::new().show(|| {
        centered_column(|| {
            colored_box_container(colors::BACKGROUND_1, || {
                Pad::all(PADDING_LARGE).show(|| {
                    centered_column(|| {
                        // Window Title Bar
                        Pad::all(PADDING_MEDIUM).show(|| {
                            row(|| {
                                heading(&title);
                            });
                        });

                        children()
                    });
                });
            });
        });
    });
}
