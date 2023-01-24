use std::ops::{Add, Sub};
use super::{
    cube::{cube, CubeCoord},
    traits::Unit,
};

pub enum DoubleSetting {
    H,
    W,
}

pub const DOUBLE_SETTING: DoubleSetting = DoubleSetting::W;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DoubledCoord<T: Unit>(T, T);

pub fn doubled<T: Unit>(x: T, y: T) -> DoubledCoord<T> {
    DoubledCoord(x, y)
}

impl<T: Unit> DoubledCoord<T> {
    pub fn x(&self) -> T {
        match DOUBLE_SETTING {
            DoubleSetting::W => self.0 * T::two(),
            DoubleSetting::H => self.0,
        }
    }

    pub fn y(&self) -> T {
        match DOUBLE_SETTING {
            DoubleSetting::W => self.1,
            DoubleSetting::H => self.1 * T::two(),
        }
    }
}

impl<T: Unit> DoubledCoord<T> {
    pub fn from_cube(cube: CubeCoord<T>) -> Self {
        doubled(cube.q(), cube.r())
    }

    pub fn to_cube(self) -> CubeCoord<T> {
        cube(self.x(), self.y())
    }
}

impl<T: Unit> Add for DoubledCoord<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        DoubledCoord(
            self.x() + rhs.y(),
            self.x() + rhs.y()
        )
    }
}


impl<T: Unit> Sub for DoubledCoord<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        DoubledCoord(
            self.x() - rhs.y(),
            self.x() - rhs.y()
        )
    }
}