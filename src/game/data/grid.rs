use std::{any::type_name, marker::PhantomData};

use cgmath::{point2, point3, vec2};

use crate::math::data::{Num, Point2, Point3, Vector2};

use super::{
    pos::{Pos, Real},
    raw::{CanBeRaw, Raw},
};

pub const SQRT_3: Num = 1.732050807568877293527446341505;

pub const SIZE_X: usize = 8;
pub const SIZE_Y: usize = 8;
pub const SIZE_Z: usize = 4;
pub const ISIZE_X: isize = SIZE_X as isize;
pub const ISIZE_Y: isize = SIZE_Y as isize;
pub const ISIZE_Z: isize = SIZE_Z as isize;
pub const NUM_X: Num = SIZE_X as Num;
pub const NUM_Y: Num = SIZE_Y as Num;
pub const NUM_Z: Num = SIZE_Z as Num;

pub const POINT_XYZ: Point3 = point3(NUM_X, NUM_Y, NUM_Z);
pub const GRID_SIZE: usize = SIZE_X * SIZE_Y * SIZE_Z;

pub const WORLD_HEX_SIZE: Num = 0.5;

pub const WORLD_X: Num = SQRT_3 * WORLD_HEX_SIZE;
pub const WORLD_Y: Num = 2.0 * WORLD_HEX_SIZE;

pub const WORLD_X_SPACING: Num = WORLD_X;
pub const WORLD_Y_SPACING: Num = WORLD_Y * 0.75;

pub const WORLD_SPACING: Vector2 = vec2(WORLD_X_SPACING, WORLD_Y_SPACING);

fn col(odd_row: Num) -> Num {
    odd_row * WORLD_X_SPACING / 2.0
}

#[inline]
pub fn real_pos_to_world(pos: Pos) -> Vector2 {
    let x = pos.0 as Num;
    let y = pos.1 as Num;
    let odd_row = pos.1 % 2;

    vec2(
        x * WORLD_X_SPACING + col(odd_row as Num),
        y * WORLD_Y_SPACING,
    )
}

#[inline]
pub fn world_pos_to_real_point(pos: Vector2) -> Point2 {
    let x = pos.x / WORLD_X_SPACING;
    let y = pos.y / WORLD_Y_SPACING;

    point2(x, y)
}

#[inline]
pub fn world_pos_to_real(pos: Vector2) -> Pos {
    let p = world_pos_to_real_point(pos);

    Pos(p.x.round() as Real, p.y.round() as Real)
}

#[derive(Debug, Clone)]
pub struct Grid<T, R>
where
    T: CanBeRaw<R>,
    R: Raw,
{
    __phantom: PhantomData<R>,

    pub data: Vec<T>,
}

pub fn to_xyz(idx: isize) -> (isize, isize, isize) {
    (
        idx % ISIZE_X,
        idx / ISIZE_X % ISIZE_Y,
        idx / ISIZE_X / ISIZE_Y % ISIZE_Z,
    )
}

impl<T, R> Grid<T, R>
where
    T: CanBeRaw<R> + Default,
    R: Raw,
{
    pub fn new(mut data: Vec<T>) -> Self {
        data.resize_with(GRID_SIZE, T::default);

        Self {
            data,
            __phantom: PhantomData,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::with_capacity(GRID_SIZE))
    }
}

impl<T: Default + Clone, R> Default for Grid<T, R>
where
    T: CanBeRaw<R>,
    R: Raw,
{
    fn default() -> Self {
        let mut data = Vec::with_capacity(GRID_SIZE);

        data.resize_with(GRID_SIZE, T::default);

        Self::new(data)
    }
}

impl<T, R> Into<Vec<u8>> for Grid<T, R>
where
    T: CanBeRaw<R> + Default + Clone,
    R: Raw,
{
    fn into(self) -> Vec<u8> {
        let mut data = self.data;

        if !(data.len() == GRID_SIZE) {
            // todo try from
            log::error!(
                "[{}] data doesn't match the size of {} before converting to bytes",
                type_name::<Self>(),
                GRID_SIZE
            );

            data.resize_with(GRID_SIZE, T::default);
        }

        data.into_iter()
            .flat_map(|v| {
                let raw: R = v.into();

                raw.to_bytes()
            })
            .collect()
    }
}
