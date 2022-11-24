use std::ops::Sub;

use num::One;
use num::Signed;
use num::Zero;

pub trait Unit: Signed + Zero + One + Copy {
    fn neg_one() -> Self {
        -Self::one()
    }
    fn two() -> Self {
        Self::one() + Self::one()
    }
}

impl Unit for i8 {}
impl Unit for i16 {}
impl Unit for i32 {}
impl Unit for i64 {}
impl Unit for isize {}
impl Unit for f32 {}
impl Unit for f64 {}

pub trait Coord<T: Unit>: Sized {
    fn length(&self) -> T;

    fn distance<'a>(&'a self, other: &'a Self) -> T
    where
        &'a Self: Sub<Output = Self>,
    {
        (other - self).length()
    }
}

pub trait Direction<T, U, const LEN: usize>: Into<U>
where
    Self: 'static + Sized + Copy,
    T: Unit,
    U: Coord<T>,
{
    const DIRECTIONS: &'static [Self; LEN];

    fn direction(dir: usize) -> Self {
        let dir = dir % LEN;

        Self::DIRECTIONS[dir]
    }

    fn signed_direction(dir: isize) -> Self {
        let dir = {
            let abs = dir.unsigned_abs();

            if !dir.is_negative() {
                abs
            } else {
                LEN - (abs % LEN)
            }
        };

        Self::direction(dir)
    }

    fn to_unit_coord(self) -> U {
        Into::into(self)
    }
}
