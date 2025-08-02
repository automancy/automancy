use std::{
    cmp::{max, min},
    fmt::{Display, Formatter},
    ops::{Add, Deref, Div, Mul, Neg, Sub},
};

use serde::{Deserialize, Serialize};

use crate::{
    math::{Float, Matrix4, Vec2, Vec3},
    rendering::camera::{WORLD_PLANE_Z, pixel_to_world, viewport_bounding_rect_in_world},
};

pub const TILE_GRID_LAYOUT: hexx::HexLayout = hexx::HexLayout {
    orientation: hexx::HexOrientation::Pointy,
    origin: Vec2::ZERO,
    scale: Vec2::ONE,
};

/// The type of number that will be stored in a tile's coordinates. Should probably be a signed integer.
pub type TileUnit = i32;

/// The basic Hex Grid coordinate.
pub type TileHex = hexx::Hex;

/// Represents a tile's position.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TileCoord(TileHex);

impl Deref for TileCoord {
    type Target = TileHex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TileHex> for TileCoord {
    fn from(value: TileHex) -> Self {
        Self(value)
    }
}

impl From<TileCoord> for TileHex {
    fn from(value: TileCoord) -> Self {
        value.0
    }
}

impl TileCoord {
    /// Shorthand for the tile at position (0, 0).
    pub const ORIGIN: Self = Self(TileHex::new(0, 0));

    /// Creates a new coordinate from a q and an r component, at the position (q, r, -q - r).
    #[must_use]
    pub const fn new(q: TileUnit, r: TileUnit) -> Self {
        Self(TileHex::new(q, r))
    }
}

impl TileCoord {
    /// Creates a minimal string of the coordinate.
    #[inline]
    #[must_use]
    pub fn to_minimal_string(self) -> String {
        format!("{},{}", self.x, self.y)
    }

    pub const TOP_RIGHT: Self = Self(hexx::EdgeDirection::POINTY_TOP_RIGHT.into_hex());
    pub const RIGHT: Self = Self(hexx::EdgeDirection::POINTY_RIGHT.into_hex());
    pub const BOTTOM_RIGHT: Self = Self(hexx::EdgeDirection::POINTY_BOTTOM_RIGHT.into_hex());
    pub const BOTTOM_LEFT: Self = Self(hexx::EdgeDirection::POINTY_BOTTOM_LEFT.into_hex());
    pub const LEFT: Self = Self(hexx::EdgeDirection::POINTY_LEFT.into_hex());
    pub const TOP_LEFT: Self = Self(hexx::EdgeDirection::POINTY_TOP_LEFT.into_hex());

    /// Creates a list of the neighbors
    #[inline]
    #[must_use]
    pub fn neighbors(self) -> [Self; 6] {
        [
            self + Self::TOP_RIGHT,
            self + Self::RIGHT,
            self + Self::BOTTOM_RIGHT,
            self + Self::BOTTOM_LEFT,
            self + Self::LEFT,
            self + Self::TOP_LEFT,
        ]
    }

    #[inline]
    #[must_use]
    pub fn as_translation(self) -> Matrix4 {
        let p = TILE_GRID_LAYOUT.hex_to_world_pos(self.0);

        Matrix4::from_translation(p.extend(WORLD_PLANE_Z))
    }
}

impl Display for TileCoord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{}, {}]", self.0.x, self.0.y))
    }
}

impl Add for TileCoord {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for TileCoord {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<TileUnit> for TileCoord {
    type Output = Self;

    fn mul(self, rhs: TileUnit) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<TileUnit> for TileCoord {
    type Output = Self;

    fn div(self, rhs: TileUnit) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Neg for TileCoord {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

/// Represents a tile's position.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TileBounds {
    #[default]
    Empty,
    Hex(hexx::HexBounds),
}

impl TileBounds {
    #[inline]
    #[must_use]
    pub const fn new(center: TileCoord, radius: u32) -> Self {
        Self::Hex(hexx::HexBounds {
            center: center.0,
            radius,
        })
    }

    #[inline]
    #[must_use]
    pub fn from_min_max(min: TileCoord, max: TileCoord) -> Self {
        let center = (min + max) / 2;
        let radius = center.unsigned_distance_to(*max) / 2;
        Self::new(center, radius)
    }

    #[inline]
    #[must_use]
    pub fn radius(&self) -> u32 {
        match self {
            TileBounds::Empty => 0,
            TileBounds::Hex(v) => v.radius,
        }
    }

    #[inline]
    #[must_use]
    pub fn center(&self) -> TileCoord {
        match self {
            TileBounds::Empty => TileCoord::ORIGIN,
            TileBounds::Hex(v) => TileCoord(v.center),
        }
    }

    #[inline]
    #[must_use]
    pub fn contains(&self, coord: TileCoord) -> bool {
        match self {
            TileBounds::Empty => false,
            TileBounds::Hex(v) => v.is_in_bounds(*coord),
        }
    }
}

impl FromIterator<TileCoord> for TileBounds {
    fn from_iter<T: IntoIterator<Item = TileCoord>>(iter: T) -> Self {
        let mut min = TileCoord::new(i32::MAX, i32::MAX);
        let mut max = TileCoord::new(i32::MIN, i32::MIN);

        for hex in iter {
            min.0.x = min.x.min(hex.x);
            max.0.x = max.x.max(hex.x);
            min.0.y = min.y.min(hex.y);
            max.0.y = max.y.max(hex.y);
        }
        Self::from_min_max(min, max)
    }
}

impl From<hexx::HexBounds> for TileBounds {
    fn from(value: hexx::HexBounds) -> Self {
        Self::Hex(value)
    }
}

#[derive(Debug, Clone)]
pub struct ExactSizeCoordIterator {
    bounds: TileBounds,
    count: usize,
    radius: TileUnit,
    x: TileUnit,
    y: Option<TileUnit>,
}

impl ExactSizeCoordIterator {
    pub fn new(bounds: TileBounds) -> Self {
        let radius = bounds.radius() as TileUnit;

        Self {
            bounds,
            count: (3 * bounds.radius() * (bounds.radius() + 1) + 1) as usize,
            radius,
            x: -radius,
            y: None,
        }
    }
}

impl Iterator for ExactSizeCoordIterator {
    type Item = TileCoord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }

        self.count = self.count.saturating_sub(1);

        let y = self
            .y
            .get_or_insert_with(|| max(-self.radius, -self.x - self.radius));

        if self.x > self.radius {
            return None;
        }

        let result = Some(self.bounds.center() + TileCoord::new(self.x, *y));
        *y += 1;

        if *y > min(self.radius, self.radius - self.x) {
            self.x += 1;
            self.y = None;
        }

        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl ExactSizeIterator for ExactSizeCoordIterator {}

impl IntoIterator for TileBounds {
    type Item = TileCoord;
    type IntoIter = ExactSizeCoordIterator;

    fn into_iter(self) -> Self::IntoIter {
        ExactSizeCoordIterator::new(self)
    }
}

#[inline]
#[must_use]
pub fn lerp_coords_to_pixel(a: TileCoord, b: TileCoord, t: Float) -> Vec2 {
    let a = Vec2::new(a.x as Float, a.y as Float);
    let b = Vec2::new(b.x as Float, b.y as Float);
    let lerp = Vec2::lerp(a, b, t);

    let p = TILE_GRID_LAYOUT.fract_hex_to_world_pos(lerp);

    Vec2::new(p.x, p.y)
}

/// Gets the hex position from a screen position, from the perspective of a camera.
#[inline]
#[must_use]
pub fn pixel_to_fract_hex(pos: Vec2, viewport_size: Vec2, camera_pos: Vec3) -> Vec2 {
    let p = pixel_to_world(pos, viewport_size, camera_pos);

    TILE_GRID_LAYOUT.world_pos_to_fract_hex(p.xy())
}

/// Gets the culling range from the camera's position
#[inline]
#[must_use]
pub fn get_culling_range(size: (Float, Float), camera_pos: Vec3) -> TileBounds {
    let (bound_min, bound_max) = viewport_bounding_rect_in_world(size, camera_pos);

    let size = bound_max - bound_min;
    let bound_center = size / 2.0 + bound_min;

    let size = TILE_GRID_LAYOUT.world_pos_to_hex((size / 2.0).ceil());

    TileBounds::new(
        TILE_GRID_LAYOUT.world_pos_to_hex(bound_center).into(),
        size.ulength(),
    )
}

#[inline]
#[must_use]
pub fn offset_coord_to_tile(a: [TileUnit; 2]) -> TileCoord {
    TileCoord::from(TileHex::from_offset_coordinates(
        a,
        hexx::OffsetHexMode::Even,
        hexx::HexOrientation::Pointy,
    ))
}

#[inline]
#[must_use]
pub fn coord_to_angle(direction: TileCoord) -> Option<Float> {
    match direction {
        TileCoord::TOP_RIGHT => Some(0.0),
        TileCoord::RIGHT => Some(-60.0),
        TileCoord::BOTTOM_RIGHT => Some(-120.0),
        TileCoord::BOTTOM_LEFT => Some(-180.0),
        TileCoord::LEFT => Some(-240.0),
        TileCoord::TOP_LEFT => Some(-300.0),
        _ => None,
    }
}
