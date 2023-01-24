use std::{
    mem::variant_count,
    ops::{Add, Div, Mul, Sub},
};

use serde::{Deserialize, Serialize};

use super::traits::{Coord, Direction, Unit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CubeCoord<T: Unit>(pub T, pub T, pub T);

impl<T: Unit> CubeCoord<T> {
    pub fn new(q: T, r: T) -> Self {
        Self::new_raw(q, r, -q - r)
    }

    pub const fn new_raw(q: T, r: T, s: T) -> Self {
        CubeCoord(q, r, s)
    }
}

pub fn cube<T: Unit>(q: T, r: T) -> CubeCoord<T> {
    CubeCoord::new(q, r)
}

impl<T: Unit> Add for CubeCoord<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let Self(q1, r1, s1) = self;
        let Self(q2, r2, s2) = rhs;

        Self::new_raw(q1 + q2, r1 + r2, s1 + s2)
    }
}

impl<T: Unit> Sub for CubeCoord<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let Self(q1, r1, s1) = self;
        let Self(q2, r2, s2) = rhs;

        Self::new_raw(q1 - q2, r1 - r2, s1 - s2)
    }
}

impl<T: Unit> Mul for CubeCoord<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let Self(q1, r1, s1) = self;
        let Self(q2, r2, s2) = rhs;

        Self::new_raw(q1 * q2, r1 * r2, s1 * s2)
    }
}

impl<T: Unit> Div for CubeCoord<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let Self(q1, r1, s1) = self;
        let Self(q2, r2, s2) = rhs;

        Self::new_raw(q1 / q2, r1 / r2, s1 / s2)
    }
}

impl<T: Unit> CubeCoord<T> {
    pub fn q(&self) -> T {
        self.0
    }

    pub fn r(&self) -> T {
        self.1
    }

    pub fn s(&self) -> T {
        self.2
    }
}

impl<T: Unit> Coord<T> for CubeCoord<T> {
    fn length(&self) -> T {
        (self.q().abs() + self.r().abs() + self.s().abs()) / T::two()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CubeDirection {
    R_,
    RU,
    LU,
    L_,
    LD,
    RD,
}

impl CubeDirection {
    pub const LEN: usize = variant_count::<Self>();

    const fn get_directions() -> [Self; Self::LEN] {
        use CubeDirection::*;

        [R_, RU, LU, L_, LD, RD]
    }

    pub const DIRECTIONS: [Self; Self::LEN] = Self::get_directions();
}

impl<T: Unit> Into<CubeCoord<T>> for CubeDirection {
    fn into(self) -> CubeCoord<T> {
        use CubeDirection::*;

        let p1 = T::one();
        let n1 = T::neg_one();
        let z0 = T::zero();

        match self {
            R_ => cube(p1, z0),
            RU => cube(p1, n1),
            LU => cube(z0, n1),
            L_ => cube(n1, z0),
            LD => cube(n1, p1),
            RD => cube(z0, n1),
        }
    }
}

impl<T: Unit> Direction<T, CubeCoord<T>, { Self::LEN }> for CubeDirection {
    const DIRECTIONS: &'static [Self; Self::LEN] = &Self::DIRECTIONS;
}
