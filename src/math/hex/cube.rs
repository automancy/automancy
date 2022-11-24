use std::{
    mem::variant_count,
    ops::{Add, Div, Mul, Sub},
};

use serde::{Deserialize, Serialize};

use super::traits::{Coord, Direction, Unit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CubeCoord<T: Unit> {
    data: [T; 3],
}

impl<T: Unit> CubeCoord<T> {
    pub fn new(q: T, r: T) -> Self {
        Self::new_raw(q, r, -q - r)
    }
}

pub fn cube<T: Unit>(q: T, r: T) -> CubeCoord<T> {
    CubeCoord::<T>::new(q, r)
}

impl<T: Unit> CubeCoord<T> {
    pub const fn new_array(data: [T; 3]) -> Self {
        Self { data }
    }

    pub const fn new_raw(q: T, r: T, s: T) -> Self {
        Self::new_array([q, r, s])
    }
}

impl_self_op!(
    (Add, Sub, Mul, Div),
    (add, sub, mul, div),
    CubeCoord<T: Unit>,
    new_raw,
    (data[0], data[1], data[2])
);

impl<T: Unit> CubeCoord<T> {
    pub fn q(&self) -> T {
        self.data[0]
    }

    pub fn r(&self) -> T {
        self.data[1]
    }

    pub fn s(&self) -> T {
        self.data[2]
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
        let _0 = T::zero();

        match self {
            R_ => cube(p1, _0),
            RU => cube(p1, n1),
            LU => cube(_0, n1),
            L_ => cube(n1, _0),
            LD => cube(n1, p1),
            RD => cube(_0, n1),
        }
    }
}

impl<T: Unit> Direction<T, CubeCoord<T>, { Self::LEN }> for CubeDirection {
    const DIRECTIONS: &'static [Self; Self::LEN] = &Self::DIRECTIONS;
}
