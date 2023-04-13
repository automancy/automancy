use crate::util::cg::Float;
use egui::Rgba;

pub trait WithAlpha {
    fn with_alpha(&self, a: Float) -> Self;
}

impl WithAlpha for Rgba {
    fn with_alpha(&self, a: Float) -> Self {
        Rgba::from_rgba_premultiplied(self.r(), self.g(), self.b(), a)
    }
}

pub const RED: Rgba = Rgba::from_rgba_premultiplied(1.0, 0.1, 0.1, 1.0);
pub const ORANGE: Rgba = Rgba::from_rgba_premultiplied(1.0, 0.745, 0.447, 1.0);
pub const WHITE: Rgba = Rgba::from_rgba_premultiplied(1.0, 1.0, 1.0, 1.0);
pub const LIGHT_GRAY: Rgba = Rgba::from_rgba_premultiplied(0.75, 0.75, 0.75, 1.0);
pub const GRAY: Rgba = Rgba::from_rgba_premultiplied(0.5, 0.5, 0.5, 1.0);
pub const DARK_GRAY: Rgba = Rgba::from_rgba_premultiplied(0.25, 0.25, 0.25, 1.0);
pub const BLACK: Rgba = Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 1.0);
pub const TRANSPARENT: Rgba = Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 0.0);
