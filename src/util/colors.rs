use std::ops::Mul;

use egui::{Color32, Rgba};

use crate::util::cg::Num;

#[derive(Debug, Copy, Clone)]
pub struct Color {
    pub r: Num,
    pub g: Num,
    pub b: Num,
    pub a: Num,
}

impl Color {
    pub fn with_alpha(self, alpha: Num) -> Self {
        Self { a: alpha, ..self }
    }
}

impl From<Color> for [Num; 3] {
    fn from(it: Color) -> Self {
        [it.r, it.g, it.b]
    }
}

impl From<Color> for [Num; 4] {
    fn from(it: Color) -> Self {
        [it.r, it.g, it.b, it.a]
    }
}

impl From<Color> for Rgba {
    fn from(it: Color) -> Self {
        Self::from_rgba_premultiplied(it.r, it.g, it.b, it.a)
    }
}

impl From<Color> for Color32 {
    fn from(it: Color) -> Self {
        let rgba: Rgba = it.into();

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
    pub const ORANGE: Color = Color {
        r: 1.0,
        g: 0.745,
        b: 0.447,
        a: 1.0,
    };
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const LIGHT_GRAY: Color = Color {
        r: 0.75,
        g: 0.75,
        b: 0.75,
        a: 1.0,
    };
    pub const GRAY: Color = Color {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    };
    pub const DARK_GRAY: Color = Color {
        r: 0.25,
        g: 0.25,
        b: 0.25,
        a: 1.0,
    };
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
}
