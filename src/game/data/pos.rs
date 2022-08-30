use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
    ops::{Add, Sub},
};

use cgmath::vec2;

use crate::math::data::{Num, Vector2};

pub type Real = isize;

#[derive(Debug, Clone, Copy, Default, Hash, Eq, Ord)]
pub struct Pos(pub Real, pub Real);

impl Pos {
    pub fn to_vec(&self) -> Vector2 {
        vec2(self.0 as Num, self.1 as Num)
    }
}

impl PartialEq<Pos> for Pos {
    fn eq(&self, other: &Pos) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl PartialOrd<Pos> for Pos {
    fn partial_cmp(&self, other: &Pos) -> Option<Ordering> {
        let x = self.0.partial_cmp(&other.0)?;
        let y = self.1.partial_cmp(&other.1)?;

        if x == y {
            return Some(x);
        } else {
            if x.is_eq() {
                return Some(y);
            } else if y.is_eq() {
                return Some(x);
            }

            if x.is_gt() {
                return Some(x);
            } else {
                return Some(y);
            }
        }
    }
}

impl Sub<Pos> for Pos {
    type Output = Self;

    fn sub(self, rhs: Pos) -> Self::Output {
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl Add<Pos> for Pos {
    type Output = Self;

    fn add(self, rhs: Pos) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Display for Pos {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{},{}",
            self.0.to_string(),
            self.1.to_string()
        ))
    }
}
