use serde::{Deserialize, Serialize};

use super::{
    cube::{cube, CubeCoord},
    traits::Unit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OffsetCoord<T: Unit> {
    data: [T; 2],
}

impl<T: Unit> OffsetCoord<T> {
    pub fn new(x: T, y: T) -> Self {
        Self::new_array([x, y])
    }
}

pub fn offset<T: Unit>(x: T, y: T) -> OffsetCoord<T> {
    OffsetCoord::<T>::new(x, y)
}

impl<T: Unit> OffsetCoord<T> {
    pub fn new_array(data: [T; 2]) -> Self {
        Self { data }
    }
}

impl<T: Unit> OffsetCoord<T> {
    pub fn x(&self) -> T {
        self.data[0]
    }

    pub fn y(&self) -> T {
        self.data[1]
    }
}

impl<T: Unit> OffsetCoord<T> {
    pub fn from_cube_as_flat_top(cube: &CubeCoord<T>) -> Self {
        let x = cube.q();
        let y = cube.r() + o(x);

        offset(x, y)
    }

    pub fn to_cube_as_flat_top(&self) -> CubeCoord<T> {
        let q = self.x();
        let r = (self.y() - q) / T::two();

        cube(q, r)
    }

    pub fn from_cube_as_pointy_top(cube: &CubeCoord<T>) -> Self {
        let y = cube.r();
        let x = cube.q() + o(y);

        offset(x, y)
    }

    pub fn to_cube_as_pointy_top(&self) -> CubeCoord<T> {
        let r = self.x();
        let q = (self.y() - r) / T::two();

        cube(q, r)
    }
}
