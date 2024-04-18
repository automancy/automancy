use automancy_defs::colors;
use yakui::{
    row,
    widgets::{Layer, Pad},
};

use super::{
    container::RoundRect, layout::centered_column, text::heading, PADDING_LARGE, PADDING_MEDIUM,
};

pub fn window(title: String, children: impl FnOnce()) {
    Layer::new().show(|| {
        centered_column(|| {
            let mut container = RoundRect::new(4.0);
            container.color = colors::BACKGROUND_1;

            container.show_children(|| {
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
