use std::ops::Div;

use super::data::{Matrix4, Num};

pub fn remap(value: Num, source_min: Num, source_max: Num, dest_min: Num, dest_max: Num) -> Num {
    dest_min + ((value - source_min) / (source_max - source_min)) * (dest_max - dest_min)
}

#[rustfmt::skip]
pub fn perspective(fovy: f32, a: f32, n: f32, f: f32) -> Matrix4 {
    let t = fovy.div(2.0).tan();
    let d = f - n;
    let m = -(f * n);

    Matrix4::new(
        1.0 / (t * a), 0.0    , 0.0  , 0.0,
        0.0          , 1.0 / t, 0.0  , 0.0,
        0.0          , 0.0    , f / d, 1.0,
        0.0          , 0.0    , m / d, 0.0,
    )
}
