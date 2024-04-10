use automancy_defs::colors;
use yakui::{
    colored_box, colored_box_container, expanded, pad, row,
    widgets::{Layer, Pad},
    Color, CrossAxisAlignment, MainAxisAlignment,
};

use super::{list::List, text::heading, ICON_SIZE, PADDING_MEDIUM};

pub fn window(title: String, children: impl FnOnce()) {
    let mut c = List::column();
    c.cross_axis_alignment = CrossAxisAlignment::Center;
    c.main_axis_alignment = MainAxisAlignment::Center;

    c.show(|| {
        let mut r = List::row();
        r.cross_axis_alignment = CrossAxisAlignment::Center;
        r.main_axis_alignment = MainAxisAlignment::Center;

        r.show(|| {
            Layer::new().show(|| {
                colored_box_container(colors::BACKGROUND, || {
                    // Window Title Bar
                    pad(Pad::all(PADDING_MEDIUM), || {
                        row(|| {
                            colored_box(Color::BLUE, ICON_SIZE); // TODO
                            expanded(|| {
                                pad(Pad::balanced(PADDING_MEDIUM, 0.0), || {
                                    heading(&title);
                                });
                            });
                            colored_box(Color::RED, ICON_SIZE);
                        });
                    });

                    children()
                });
            });
        });
    });
}
