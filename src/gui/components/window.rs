use automancy_defs::colors;
use yakui::{colored_box_container, pad, row, widgets::Pad};

use super::{layer::Layer, layout::centered_column, text::heading, PADDING_LARGE, PADDING_MEDIUM};

pub fn window(title: String, children: impl FnOnce()) {
    Layer::new().show(|| {
        centered_column(|| {
            colored_box_container(colors::BACKGROUND, || {
                Pad::all(PADDING_LARGE).show(|| {
                    centered_column(|| {
                        // Window Title Bar
                        pad(Pad::all(PADDING_MEDIUM), || {
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
