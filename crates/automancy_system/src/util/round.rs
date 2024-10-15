pub trait Round {
    fn generic_round(self) -> Self;
    fn to_f64(self) -> f64;
    fn from_f64(v: f64) -> Self;
}

macro_rules! impl_integer_round {
    ($type: ty) => {
        impl Round for $type {
            fn generic_round(self) -> Self {
                self
            }

            fn to_f64(self) -> f64 {
                self as f64
            }

            fn from_f64(v: f64) -> Self {
                v as Self
            }
        }
    };
}

impl_integer_round!(i8);
impl_integer_round!(i16);
impl_integer_round!(i32);
impl_integer_round!(i64);
impl_integer_round!(i128);
impl_integer_round!(u8);
impl_integer_round!(u16);
impl_integer_round!(u32);
impl_integer_round!(u64);
impl_integer_round!(u128);

macro_rules! impl_float_round {
    ($type: ty) => {
        impl Round for $type {
            fn generic_round(self) -> Self {
                self.round()
            }

            fn to_f64(self) -> f64 {
                self as f64
            }

            fn from_f64(v: f64) -> Self {
                v as Self
            }
        }
    };
}

impl_float_round!(f32);
impl_float_round!(f64);
