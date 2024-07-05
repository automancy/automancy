use std::{
    fmt::Debug,
    ops::{Add, Div, Mul, Neg, Sub},
};

use super::round::Round;

pub trait NumTrait:
    'static
    + Round
    + Copy
    + Debug
    + PartialEq
    + PartialOrd
    + Neg<Output = Self>
    + Sub<Output = Self>
    + Add<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
{
    fn zero() -> Self;
    fn one() -> Self;
}

impl NumTrait for f32 {
    fn zero() -> Self {
        0.0
    }

    fn one() -> Self {
        1.0
    }
}

impl NumTrait for f64 {
    fn zero() -> Self {
        0.0
    }

    fn one() -> Self {
        1.0
    }
}

impl NumTrait for i32 {
    fn zero() -> Self {
        0
    }

    fn one() -> Self {
        1
    }
}

impl NumTrait for i64 {
    fn zero() -> Self {
        0
    }

    fn one() -> Self {
        1
    }
}
