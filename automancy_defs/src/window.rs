use egui::{pos2, Rect};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::math::{Double, Float};

pub fn window_size_rect(window: &Window) -> Rect {
    let (w, h) = window_size_double(window);

    Rect::from_min_max(pos2(0.0, 0.0), pos2(w as Float, h as Float))
}

pub fn window_size_double(window: &Window) -> (Double, Double) {
    let PhysicalSize { width, height } = window.inner_size();

    (width as Double, height as Double)
}

pub fn window_aspect(window: &Window) -> Double {
    let PhysicalSize { width, height } = window.inner_size();

    width as Double / height as Double
}
