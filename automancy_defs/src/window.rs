use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::math::Float;

pub fn window_size_double(window: &Window) -> (Float, Float) {
    let PhysicalSize { width, height } = window.inner_size();

    (width as Float, height as Float)
}

pub fn window_aspect(window: &Window) -> Float {
    let PhysicalSize { width, height } = window.inner_size();

    width as Float / height as Float
}
