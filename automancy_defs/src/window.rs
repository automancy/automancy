use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::math::Double;

pub fn window_size_double(window: &Window) -> (Double, Double) {
    let PhysicalSize { width, height } = window.inner_size();

    (width as Double, height as Double)
}

pub fn window_aspect(window: &Window) -> Double {
    let PhysicalSize { width, height } = window.inner_size();

    width as Double / height as Double
}
