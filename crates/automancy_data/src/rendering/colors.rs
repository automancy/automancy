use crate::math::Float;

/// An [`vek::Rgba`] type of floating point component, in linear RGB.
///
/// Note: **Is not premultiplied!**
pub type Rgba = vek::Rgba<Float>;

/// An [`vek::Rgba`] type of `u8` component, in sRGB.
///
/// Note: **Is not premultiplied!**
pub type SRgbaU8 = vek::Rgba<u8>;

pub type PackedRgba = u32;

pub const trait ColorExt {
    type ComponentType;

    #[must_use]
    fn mul_alpha(self, a: Self::ComponentType) -> Self;
    #[must_use]
    fn with_alpha(self, a: Self::ComponentType) -> Self;
    #[must_use]
    fn adjust(self, v: Float) -> Self;

    #[must_use]
    fn from_srgb_u8(color: SRgbaU8) -> Self;
    #[must_use]
    fn to_srgb_u8(self) -> SRgbaU8;
    #[must_use]
    fn to_packed(self) -> PackedRgba;
}

impl const ColorExt for Rgba {
    type ComponentType = Float;

    fn mul_alpha(mut self, a: Float) -> Self {
        self.a *= a;
        self
    }
    fn with_alpha(mut self, a: Float) -> Self {
        self.a = a;
        self
    }
    fn adjust(self, v: Float) -> Self {
        Rgba {
            r: self.r * v,
            g: self.g * v,
            b: self.b * v,
            a: self.a,
        }
    }

    fn from_srgb_u8(color: SRgbaU8) -> Self {
        Rgba {
            r: fast_srgb8::srgb8_to_f32(color.r),
            g: fast_srgb8::srgb8_to_f32(color.g),
            b: fast_srgb8::srgb8_to_f32(color.b),
            a: (color.a as Float) / 255.0,
        }
    }

    fn to_srgb_u8(self) -> SRgbaU8 {
        SRgbaU8 {
            r: fast_srgb8::f32_to_srgb8(self.r),
            g: fast_srgb8::f32_to_srgb8(self.g),
            b: fast_srgb8::f32_to_srgb8(self.b),
            a: (self.a * 255.0) as u8,
        }
    }

    fn to_packed(self) -> PackedRgba {
        let r = (self.r * 255.0) as u8;
        let g = (self.g * 255.0) as u8;
        let b = (self.b * 255.0) as u8;
        let a = (self.a * 255.0) as u8;

        PackedRgba::from_le_bytes([r, g, b, a])
    }
}

/// Turns a [`SRgbaU8`] into a [`String`] by encoding it as a hex number.
#[must_use]
pub fn encode_srgb_u8(v: SRgbaU8) -> String {
    const_hex::encode([v.r, v.g, v.b, v.a])
}

/// Turns a [`Rgba`] into a [`String`] by converting it to a [`SRgbaU8`] first, then encoding it as a hex number.
#[must_use]
pub fn encode_rgba(v: Rgba) -> String {
    encode_srgb_u8(v.to_srgb_u8())
}

/// Turns a `&str` into an [`SRgbaU8`] by interpreting it as either a 3-digits hex number or a 4-digits hex number.
/// Also strips `#` and `0x` from input.
///
/// Prefer [`const_decode_srgb_u8`] for const situations.
#[must_use]
pub fn decode_srgb_u8(s: &str) -> SRgbaU8 {
    let mut color = match const_hex::decode(s.strip_prefix("#").unwrap_or(s)) {
        Ok(v) => v.into_iter(),
        Err(_) => {
            return SRgbaU8::broadcast(0);
        },
    };

    SRgbaU8 {
        r: color.next().unwrap_or(0),
        g: color.next().unwrap_or(0),
        b: color.next().unwrap_or(0),
        a: color.next().unwrap_or(255),
    }
}

/// Turns a `&str` into an [`Rgba`] by interpreting it as either a 3-digits hex number or a 4-digits hex number.
/// Also strips `#` and `0x` from input.
///
/// Prefer [`const_decode_rgba`] for const situations.
#[must_use]
pub fn decode_rgba(s: &str) -> Rgba {
    Rgba::from_srgb_u8(decode_srgb_u8(s))
}

/// Turns a `&'static str` into an [`SRgbaU8`] by interpreting it as either a 3-digits hex number or a 4-digits hex number.
/// Also strips `#` and `0x` from input.
///
/// Prefer [`decode_srgb_u8`] for non-const situations.
#[must_use]
pub const fn const_decode_srgb_u8(s: &'static str) -> SRgbaU8 {
    if s.is_empty() {
        return SRgbaU8::broadcast(0);
    }

    let bytes = s.as_bytes();
    let bytes = match bytes {
        [b'#', rest @ ..] => rest,
        _ => bytes,
    };

    if let Ok(array) = const_hex::const_decode_to_array::<3>(bytes) {
        SRgbaU8 {
            r: array[0],
            g: array[1],
            b: array[2],
            a: 255,
        }
    } else if let Ok(array) = const_hex::const_decode_to_array::<4>(bytes) {
        SRgbaU8 {
            r: array[0],
            g: array[1],
            b: array[2],
            a: array[3],
        }
    } else {
        SRgbaU8::broadcast(0)
    }
}

/// Turns a `&'static str` into an [`Rgba`] by interpreting it as either a 3-digits hex number or a 4-digits hex number.
/// Also strips `#` and `0x` from input.
///
/// Prefer [`decode_rgba`] for non-const situations.
#[must_use]
pub const fn const_decode_rgba(s: &'static str) -> Rgba {
    Rgba::from_srgb_u8(const_decode_srgb_u8(s))
}

#[derive(Debug, Clone, Copy)]
pub struct ComplexRgba {
    pub linear: Rgba,
    pub srgb: SRgbaU8,
    pub packed: PackedRgba,
}

impl ComplexRgba {
    pub const fn from_srgb(srgb: SRgbaU8) -> Self {
        let linear = Rgba::from_srgb_u8(srgb);
        let packed = linear.to_packed();

        ComplexRgba {
            linear,
            srgb,
            packed,
        }
    }

    pub const fn from_str(s: &'static str) -> Self {
        Self::from_srgb(const_decode_srgb_u8(s))
    }

    pub const fn with_alpha_packed(self, a: u8) -> PackedRgba {
        let [r, g, b, _] = self.packed.to_le_bytes();

        PackedRgba::from_le_bytes([r, g, b, a])
    }
}

pub const TRANSPARENT: ComplexRgba = ComplexRgba::from_str("#00000000");

pub const BACKGROUND_1: ComplexRgba = ComplexRgba::from_str("#ffffffaa");
pub const BACKGROUND_2: ComplexRgba = ComplexRgba::from_str("#d4d4d4");
pub const BACKGROUND_3: ComplexRgba = ComplexRgba::from_str("#a0a0a0");
pub const BACKGROUND_OPAQUE: ComplexRgba = ComplexRgba::from_str("#ffffff");
pub const BACKGROUND_INACTIVE: ComplexRgba = ComplexRgba::from_str("#7e7e7eaa");

pub const INTERACTIVE_1: ComplexRgba = ComplexRgba::from_str("#d5bbff");
pub const INTERACTIVE_2: ComplexRgba = ComplexRgba::from_str("#c891fc");
pub const INTERACTIVE_3: ComplexRgba = ComplexRgba::from_str("#c86ffc");

pub const BUTTON_HOVERED: ComplexRgba = ComplexRgba::from_str("#d6e4ff");
pub const BUTTON_PRESSED: ComplexRgba = ComplexRgba::from_str("#becbe7");

pub const TEXT_ACTIVE: ComplexRgba = ComplexRgba::from_str("#000000");
pub const TEXT_INACTIVE: ComplexRgba = ComplexRgba::from_str("#707070");
pub const TEXT_WARNING: ComplexRgba = ComplexRgba::from_str("#8f620e");

pub const TEXT_BG_SELECTED: ComplexRgba = ComplexRgba::from_str("#c0f2ff");

pub const INPUT: ComplexRgba = ComplexRgba::from_str("#61d0ff");
pub const OUTPUT: ComplexRgba = ComplexRgba::from_str("#ff9f50");
pub const IMPORTANT: ComplexRgba = ComplexRgba::from_str("#ff2e2e");
