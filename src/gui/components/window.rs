use automancy_defs::colors;
use yakui::{
    row,
    widgets::{Layer, Pad},
};

use super::{
    container::RoundRect, layout::centered_column, text::heading, PADDING_LARGE, PADDING_SMALL,
};

pub fn window(title: String, children: impl FnOnce()) {
    Layer::new().show(|| {
        centered_column(|| {
            RoundRect::new(4.0, colors::BACKGROUND_1).show_children(|| {
                Pad::all(PADDING_LARGE).show(|| {
                    centered_column(|| {
                        // Window Title Bar
                        Pad::vertical(PADDING_SMALL).show(|| {
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
