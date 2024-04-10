use automancy_defs::colors;
use yakui::{colored_box_container, pad, widgets::Pad};

pub fn group(children: impl FnOnce()) {
    colored_box_container(colors::GRAY, || {
        pad(Pad::all(2.0), || {
            colored_box_container(colors::BACKGROUND, children);
        });
    });
}
