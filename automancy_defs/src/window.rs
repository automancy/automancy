use egui::{pos2, Rect};
use winit::window::Window;

use crate::math::{Double, Float};

pub fn window_size_rect(window: &Window) -> Rect {
    let (w, h) = window_size_double(window);
    let (w, h) = (w / window.scale_factor(), h / window.scale_factor());

    Rect::from_min_max(pos2(0.0, 0.0), pos2(w as Float, h as Float))
}

pub fn window_size_double(window: &Window) -> (Double, Double) {
    let (w, h) = window_size_u32(window);

    (w as Double, h as Double)
}

pub fn window_size_float(window: &Window) -> (Float, Float) {
    let (w, h) = window_size_u32(window);

    (w as Float, h as Float)
}

pub fn window_size_u32(window: &Window) -> (u32, u32) {
    window.inner_size().into()
}
