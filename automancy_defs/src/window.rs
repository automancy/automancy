use egui::{pos2, Rect};
use winit::window::Window;

use crate::math::{Double, Float};

pub fn window_size_rect(window: &Window) -> Rect {
    let (width, height) = window_size_float(window);

    Rect::from_min_max(pos2(0.0, 0.0), pos2(width, height))
}

pub fn window_size_double(window: &Window) -> (Double, Double) {
    window.inner_size().cast::<Double>().into()
}

pub fn window_size_float(window: &Window) -> (Float, Float) {
    window.inner_size().cast::<Float>().into()
}

pub fn window_size_u32(window: &Window) -> (u32, u32) {
    window.inner_size().into()
}
