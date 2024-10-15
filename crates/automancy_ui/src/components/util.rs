use automancy_defs::math::Vec2;
use yakui::widgets::Pad;
use yakui::Rect;

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

pub fn clamp_percentage_to_viewport(size: Vec2, mut pos: Vec2, viewport: Rect) -> Vec2 {
    let mut rect = Rect::from_pos_size((pos * viewport.size()).floor(), size);

    constrain_to_viewport(&mut rect, viewport);

    pos = (rect.pos() / viewport.size()).clamp(Vec2::ZERO, Vec2::ONE);

    pos
}
