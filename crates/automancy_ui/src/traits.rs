use automancy_data::{
    math::{UVec2, Vec2},
    rendering::colors::{ColorExt, ComplexRgba, Rgba, SRgbaU8},
};

pub const trait IntoYakui {
    type YakuiType;

    #[must_use]
    fn yak(self) -> Self::YakuiType;
}

pub const trait IntoOurs {
    type OurType;

    #[must_use]
    fn unyak(self) -> Self::OurType;
}

impl const IntoYakui for SRgbaU8 {
    type YakuiType = yakui::Color;

    fn yak(self) -> Self::YakuiType {
        Self::YakuiType {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

impl const IntoYakui for Rgba {
    type YakuiType = <SRgbaU8 as IntoYakui>::YakuiType;

    fn yak(self) -> Self::YakuiType {
        self.to_srgb_u8().yak()
    }
}

impl const IntoYakui for ComplexRgba {
    type YakuiType = yakui::Color;

    fn yak(self) -> Self::YakuiType {
        self.srgb.yak()
    }
}

impl const IntoYakui for Vec2 {
    type YakuiType = yakui::Vec2;

    fn yak(self) -> Self::YakuiType {
        yakui::Vec2::new(self.x, self.y)
    }
}

impl const IntoOurs for yakui::Vec2 {
    type OurType = Vec2;

    fn unyak(self) -> Self::OurType {
        Vec2::new(self.x, self.y)
    }
}

impl const IntoYakui for UVec2 {
    type YakuiType = yakui::UVec2;

    fn yak(self) -> Self::YakuiType {
        yakui::UVec2::new(self.x, self.y)
    }
}

impl const IntoOurs for yakui::UVec2 {
    type OurType = UVec2;

    fn unyak(self) -> Self::OurType {
        UVec2::new(self.x, self.y)
    }
}

pub trait RoundExt {
    #[must_use]
    fn round_to_step(self, step: Self) -> Self;
    #[must_use]
    fn clamp(self, min: Self, max: Self) -> Self;
}

macro_rules! impl_round_to_step_int {
    (
        $($ty:ident),*
    ) => {
        $(
            impl RoundExt for $ty {
                fn round_to_step(self, step: Self) -> Self {
                    if step == 0 { self } else { (self / step) * step }
                }
                fn clamp(self, min: Self, max: Self,) -> Self {
                    Ord::clamp(self, min, max)
                }
            }
        )*
    };
}

macro_rules! impl_round_to_step_float {
    (
        $($ty:ident),*
    ) => {
        $(
            impl RoundExt for $ty {
                fn round_to_step(self, step: Self) -> Self {
                    if step == 0.0 { self } else { (self / step).floor() * step }
                }
                fn clamp(self, min: Self, max: Self,) -> Self {
                    self.clamp(min, max)
                }
            }
        )*
    };
}

impl_round_to_step_int!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize);
impl_round_to_step_float!(f32, f64);
