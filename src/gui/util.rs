use yakui::widgets::Pad;

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
