use yakui::{layout::LayoutDom, widgets::Pad, Rect, Vec2};

pub fn pad_y(top: f32, bottom: f32) -> Pad {
    let mut pad = Pad::ZERO;
    pad.top = top;
    pad.bottom = bottom;

    pad
}

pub fn pad_x(left: f32, right: f32) -> Pad {
    let mut pad = Pad::ZERO;
    pad.left = left;
    pad.right = right;

    pad
}

pub fn constrain_to_viewport(rect: &mut Rect, layout: &LayoutDom) {
    rect.set_pos(rect.pos() - (rect.max() - layout.viewport().max()).max(Vec2::ZERO))
}
