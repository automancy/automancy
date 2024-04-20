use yakui::{widgets::Pad, Rect, Vec2};

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

pub fn constrain_to_viewport(rect: &mut Rect, viewport: Rect) {
    rect.set_pos(rect.pos() - (rect.max() - viewport.max()).max(Vec2::ZERO))
}
