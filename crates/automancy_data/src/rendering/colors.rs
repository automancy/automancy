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
    type ComponentType;

    fn mul_alpha(self, a: Self::ComponentType) -> Self;
    fn with_alpha(self, a: Self::ComponentType) -> Self;

    fn from_u8(color: RgbaU8) -> Self;
    fn to_u8(self) -> RgbaU8;

    fn encode(self) -> String;
    fn decode(v: String) -> Self;
}

impl ColorExt for Rgba {
    type ComponentType = Float;

    fn mul_alpha(mut self, a: Float) -> Self {
        self.a *= a;
        self
    }

    fn with_alpha(mut self, a: Float) -> Self {
        self.a = a;
        self
    }

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

    fn encode(self) -> String {
        let v = self.to_u8();

        const_hex::encode([v.r, v.g, v.b, v.a])
    }

    fn decode(v: String) -> Self {
        let mut color = const_hex::decode(v).unwrap_or_else(|_| vec![0, 0, 0, 255]).into_iter();

        Rgba::from_u8(RgbaU8 {
            r: color.next().unwrap_or(0),
            g: color.next().unwrap_or(0),
            b: color.next().unwrap_or(0),
            a: color.next().unwrap_or(255),
        })
    }
}

/// Turns a static str into an [Rgba] by interpreting it as either a 3-digits hex number or a 4-digits hex number.
/// Also strips '#' from input.
#[must_use]
const fn hex_color(s: &'static str) -> Rgba {
    if s.is_empty() {
        return Rgba::broadcast(0.0);
    }

    let bytes = s.as_bytes();
    let bytes = match bytes {
        [b'#', rest @ ..] => rest,
        _ => bytes,
    };

    if let Ok(array) = const_hex::const_decode_to_array::<3>(bytes) {
        rgba_from_u8(RgbaU8 {
            r: array[0],
            g: array[1],
            b: array[2],
            a: 255,
        })
    } else if let Ok(array) = const_hex::const_decode_to_array::<4>(bytes) {
        rgba_from_u8(RgbaU8 {
            r: array[0],
            g: array[1],
            b: array[2],
            a: array[3],
        })
    } else {
        Rgba::broadcast(0.0)
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
