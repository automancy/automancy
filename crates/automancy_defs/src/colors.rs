use crate::math::Vec4;
use yakui::Color;

macro_rules! hex_color {
    ($s:literal) => {{
        let array = color_hex::color_from_hex!($s);

        if array.len() == 3 {
            yakui::Color {
                r: array[0],
                g: array[1],
                b: array[2],
                a: 255,
            }
        } else if array.len() == 4 {
            #[allow(clippy::out_of_bounds_indexing)]
            yakui::Color {
                r: array[0],
                g: array[1],
                b: array[2],
                a: array[3],
            }
        } else {
            yakui::Color::BLACK
        }
    }};
}

pub trait ColorExt {
    fn to_vec4(self) -> Vec4;
}

impl ColorExt for Color {
    fn to_vec4(self) -> Vec4 {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;
        let a = self.a as f32 / 255.0;

        Vec4::new(r, g, b, a)
    }
}

pub const RED: Color = hex_color!("#ff0000");
pub const ORANGE: Color = hex_color!("#ffa160");
pub const LIGHT_BLUE: Color = hex_color!("#c2fffe");
pub const WHITE: Color = hex_color!("#ffffff");
pub const LIGHT_GRAY: Color = hex_color!("#d5d5d5");
pub const GRAY: Color = hex_color!("#747474");
pub const DARK_GRAY: Color = hex_color!("#474747");
pub const BLACK: Color = hex_color!("#000000");
pub const TRANSPARENT: Color = hex_color!("#00000000");

pub const BACKGROUND_1: Color = hex_color!("#ffffff66");
pub const BACKGROUND_2: Color = hex_color!("#cccccc");
pub const BACKGROUND_3: Color = hex_color!("#bbbbbb");
pub const INACTIVE: Color = hex_color!("#9a9a9a70");
pub const TEXT_INACTIVE: Color = hex_color!("#9a9a9a");

pub const INPUT: Color = hex_color!("#44c8ff");
pub const OUTPUT: Color = hex_color!("#ff9844");
