use std::{
    fmt::{Debug, Display},
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
    + Display
{
    fn zero() -> Self;
    fn one() -> Self;
    fn clamp(self, min: Self, max: Self) -> Self;
}

impl NumTrait for f32 {
    fn zero() -> Self {
        0.0
    }

    fn one() -> Self {
        1.0
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl NumTrait for f64 {
    fn zero() -> Self {
        0.0
    }

    fn one() -> Self {
        1.0
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl NumTrait for i32 {
    fn zero() -> Self {
        0
    }

    fn one() -> Self {
        1
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}

impl NumTrait for i64 {
    fn zero() -> Self {
        0
    }

    fn one() -> Self {
        1
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}
