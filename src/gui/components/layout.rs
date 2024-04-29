use automancy_defs::glam::vec2;
use yakui::{constrained, widgets::List, Constraints, CrossAxisAlignment, MainAxisAlignment};

pub fn centered_row(children: impl FnOnce()) {
    let mut r = List::row();
    r.cross_axis_alignment = CrossAxisAlignment::Center;
    r.main_axis_alignment = MainAxisAlignment::Center;

    r.show(children);
}

pub fn centered_column(children: impl FnOnce()) {
    let mut c = List::column();
    c.cross_axis_alignment = CrossAxisAlignment::Center;
    c.main_axis_alignment = MainAxisAlignment::Center;

    c.show(children);
}

pub fn stretch_column(width: f32, children: impl FnOnce()) {
    let mut c = List::column();
    c.cross_axis_alignment = CrossAxisAlignment::Stretch;

    constrained(Constraints::loose(vec2(width, f32::INFINITY)), || {
        c.show(children);
    });
}
