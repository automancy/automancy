use std::ops::Div;

use super::cg::{Matrix4, Num};

#[rustfmt::skip]
pub fn perspective(fov_y: Num, a: Num, n: Num, f: Num) -> Matrix4 {
    let t = fov_y.div(2.0).tan();
    let d = f - n;
    let m = -(f * n);

    Matrix4::new(
        1.0 / (t * a), 0.0    , 0.0  , 0.0,
        0.0          , 1.0 / t, 0.0  , 0.0,
        0.0          , 0.0    , f / d, 1.0,
        0.0          , 0.0    , m / d, 0.0,
    )
}
