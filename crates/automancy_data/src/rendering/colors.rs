use crate::math::Float;

pub type Rgba = vek::Rgba<Float>;
pub type RgbaU8 = vek::Rgba<u8>;

pub const fn rgba_from_u8(color: RgbaU8) -> Rgba {
    Rgba {
        r: color.r as Float / 255.0,
        g: color.g as Float / 255.0,
        b: color.b as Float / 255.0,
        a: color.a as Float / 255.0,
    }
}

pub trait ColorExt {
    fn from_u8(color: RgbaU8) -> Self;
    fn to_u8(self) -> RgbaU8;
}

impl ColorExt for Rgba {
    fn from_u8(color: RgbaU8) -> Self {
        rgba_from_u8(color)
    }

    fn to_u8(self) -> RgbaU8 {
        RgbaU8 {
            r: (self.r * 255.0) as u8,
            g: (self.g * 255.0) as u8,
            b: (self.b * 255.0) as u8,
            a: (self.a * 255.0) as u8,
        }
    }
}

const fn hex_color(s: &'static str) -> Rgba {
    if let Ok(array) = const_hex::const_decode_to_array::<3>(s.as_bytes()) {
        rgba_from_u8(RgbaU8 {
            r: array[0],
            g: array[1],
            b: array[2],
            a: 255,
        })
    } else if let Ok(array) = const_hex::const_decode_to_array::<4>(s.as_bytes()) {
        rgba_from_u8(RgbaU8 {
            r: array[0],
            g: array[1],
            b: array[2],
            a: array[3],
        })
    } else {
        Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    }
}

pub const RED: Rgba = hex_color("#ff0000");
pub const ORANGE: Rgba = hex_color("#ffa160");
pub const LIGHT_BLUE: Rgba = hex_color("#c2fffe");
pub const WHITE: Rgba = hex_color("#ffffff");
pub const LIGHT_GRAY: Rgba = hex_color("#d5d5d5");
pub const GRAY: Rgba = hex_color("#747474");
pub const DARK_GRAY: Rgba = hex_color("#474747");
pub const BLACK: Rgba = hex_color("#000000");
pub const TRANSPARENT: Rgba = hex_color("#00000000");

pub const BACKGROUND_1: Rgba = hex_color("#ffffff66");
pub const BACKGROUND_2: Rgba = hex_color("#cccccc");
pub const BACKGROUND_3: Rgba = hex_color("#bbbbbb");
pub const INACTIVE: Rgba = hex_color("#9a9a9a70");
pub const TEXT_INACTIVE: Rgba = hex_color("#9a9a9a");

pub const INPUT: Rgba = hex_color("#44c8ff");
pub const OUTPUT: Rgba = hex_color("#ff9844");
