use std::ops::Mul;
use crate::math::cg::Num;

#[derive(Debug, Copy, Clone)]
pub struct Color {
    pub r: Num,
    pub g: Num,
    pub b: Num,
    pub a: Num,
}

impl Color {
    pub fn with_alpha(self, alpha: Num) -> Self {
        Self {
            a: alpha,
            ..self
        }
    }
}

impl Into<[Num; 3]> for Color {
    fn into(self) -> [Num; 3] {
        [self.r, self.g, self.b]
    }
}

impl Into<[Num; 4]> for Color {
    fn into(self) -> [Num; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Into<egui::Rgba> for Color {
    fn into(self) -> egui::Rgba {
        egui::Rgba::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
}

impl Into<egui::Color32> for Color {
    fn into(self) -> egui::Color32 {
        let rgba: egui::Rgba = self.into();

        rgba.into()
    }
}

impl Mul<Num> for Color {
    type Output = Color;

    fn mul(self, rhs: Num) -> Self::Output {
        Self {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            ..self
        }
    }
}

impl Color {
    pub const ORANGE: Color
        = Color { r: 1.0, g: 0.745, b: 0.447, a: 1.0 };
    pub const WHITE: Color
        = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const GRAY: Color
        = Color { r: 0.7, g: 0.7, b: 0.7, a: 1.0 };
    pub const TRANSPARENT: Color
        = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
}