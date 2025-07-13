use std::{
    cmp::{max, min},
    fmt::{Display, Formatter},
    ops::{Add, Deref, Div, Mul, Neg, Sub},
};

use serde::{Deserialize, Serialize};

use crate::{
    math::{Float, Int, Matrix4, Vec2, Vec3, consts},
    rendering,
};

const TILE_GRID_LAYOUT: hexx::HexLayout = hexx::HexLayout {
    orientation: hexx::HexOrientation::Pointy,
    origin: hexx::Vec2::ZERO,
    scale: hexx::Vec2::ONE,
};

/// The type that will be used to represent a tile's coordinates.
pub type TileUnit = Int;

/// The inner hex coordinate.
pub type TileHex = hexx::Hex;

/// A fractional hex coordinate
pub type FractHex = Vec2;

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

    pub const TOP_RIGHT: TileCoord = TileCoord(hexx::EdgeDirection::POINTY_TOP_RIGHT.into_hex());
    pub const TOP_LEFT: TileCoord = TileCoord(hexx::EdgeDirection::POINTY_TOP_LEFT.into_hex());
    pub const LEFT: TileCoord = TileCoord(hexx::EdgeDirection::POINTY_LEFT.into_hex());
    pub const BOTTOM_LEFT: TileCoord =
        TileCoord(hexx::EdgeDirection::POINTY_BOTTOM_LEFT.into_hex());
    pub const BOTTOM_RIGHT: TileCoord =
        TileCoord(hexx::EdgeDirection::POINTY_BOTTOM_RIGHT.into_hex());
    pub const RIGHT: TileCoord = TileCoord(hexx::EdgeDirection::POINTY_RIGHT.into_hex());

    /// Creates a list of the neighbors
    #[inline]
    #[must_use]
    pub fn neighbors(self) -> [TileCoord; 6] {
        [
            self + TileCoord::TOP_RIGHT,
            self + TileCoord::TOP_LEFT,
            self + TileCoord::LEFT,
            self + TileCoord::BOTTOM_LEFT,
            self + TileCoord::BOTTOM_RIGHT,
            self + TileCoord::RIGHT,
        ]
    }

    #[inline]
    #[must_use]
    pub fn to_world_pos(self) -> Vec2 {
        let p = TILE_GRID_LAYOUT.hex_to_world_pos(*self);

        Vec2::new(p.x, p.y)
    }

    #[inline]
    #[must_use]
    pub fn from_world_pos(pos: Vec2) -> TileCoord {
        TileCoord(TILE_GRID_LAYOUT.world_pos_to_hex(hexx::Vec2::new(pos.x, pos.y)))
    }

    #[inline]
    #[must_use]
    pub fn from_offset_coord(coord: [TileUnit; 2]) -> TileCoord {
        TileCoord::from(TileHex::from_offset_coordinates(
            coord,
            hexx::OffsetHexMode::Even,
            hexx::HexOrientation::Pointy,
        ))
    }

    #[inline]
    #[must_use]
    pub fn round(fract: FractHex) -> TileCoord {
        TileCoord(TileHex::round(fract.into_array()))
    }

    #[inline]
    #[must_use]
    pub fn fract_hex_to_world(hex: FractHex) -> Vec2 {
        let pos = TILE_GRID_LAYOUT.fract_hex_to_world_pos(hexx::Vec2::new(hex.x, hex.y));

        Vec2::new(pos.x, pos.y)
    }

    #[inline]
    #[must_use]
    pub fn world_to_fract_hex(pos: Vec2) -> FractHex {
        let hex = TILE_GRID_LAYOUT.world_pos_to_fract_hex(hexx::Vec2::new(pos.x, pos.y));

        FractHex::new(hex.x, hex.y)
    }

    #[inline]
    #[must_use]
    pub fn lerp_coords(a: TileCoord, b: TileCoord, t: Float) -> FractHex {
        let a = Vec2::new(a.x as Float, a.y as Float);
        let b = Vec2::new(b.x as Float, b.y as Float);

        FractHex::lerp(a, b, t)
    }

    #[inline]
    #[must_use]
    pub fn as_translation(self) -> Matrix4 {
        Matrix4::translation_3d(self.to_world_pos())
    }

    #[inline]
    #[must_use]
    pub fn world_pos_to_degrees(coord: Vec2) -> Float {
        let rad = (-coord.y).atan2(coord.x);

        (rad - consts::FRAC_PI_3)
            .rem_euclid(consts::TAU)
            .to_degrees()
            .round()
    }

    #[inline]
    #[must_use]
    pub fn as_degrees(self) -> Float {
        match self {
            TileCoord::TOP_RIGHT => 0.0,
            TileCoord::TOP_LEFT => 60.0,
            TileCoord::LEFT => 120.0,
            TileCoord::BOTTOM_LEFT => 180.0,
            TileCoord::BOTTOM_RIGHT => 240.0,
            TileCoord::RIGHT => 300.0,
            _ => TileCoord::world_pos_to_degrees(self.to_world_pos()),
        }
    }
}

impl Display for TileCoord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{}, {}]", self.0.x, self.0.y))
    }
}

impl Add for TileCoord {
    type Output = TileCoord;

    fn add(self, rhs: TileCoord) -> Self::Output {
        TileCoord(self.0 + rhs.0)
    }
}

impl Sub for TileCoord {
    type Output = TileCoord;

    fn sub(self, rhs: TileCoord) -> Self::Output {
        TileCoord(self.0 - rhs.0)
    }
}

impl Mul<TileUnit> for TileCoord {
    type Output = TileCoord;

    fn mul(self, rhs: TileUnit) -> Self::Output {
        TileCoord(self.0 * rhs)
    }
}

impl Mul<TileCoord> for TileUnit {
    type Output = TileCoord;

    fn mul(self, rhs: TileCoord) -> Self::Output {
        TileCoord(rhs.0 * self)
    }
}

impl Div<TileUnit> for TileCoord {
    type Output = TileCoord;

    fn div(self, rhs: TileUnit) -> Self::Output {
        TileCoord(self.0 / rhs)
    }
}

impl Neg for TileCoord {
    type Output = TileCoord;

    fn neg(self) -> Self::Output {
        TileCoord(-self.0)
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

    /// Gets the culling range from the camera's position
    #[inline]
    #[must_use]
    pub fn from_display(size: Vec2, camera_pos: Vec3) -> TileBounds {
        let (bound_min, bound_max) =
            rendering::camera::viewport_bounding_rect_in_world(size, camera_pos);

        let size = bound_max - bound_min;
        let bound_center = size / 2.0 + bound_min;

        let size = TileCoord::from_world_pos((size / Vec2::new(consts::SQRT_3, 2.0)).ceil());
        let center = TileCoord::from_world_pos(bound_center);

        TileBounds::new(center, size.ulength())
    }
}

impl FromIterator<TileCoord> for TileBounds {
    fn from_iter<T: IntoIterator<Item = TileCoord>>(iter: T) -> Self {
        let mut min = TileCoord::new(Int::MAX, Int::MAX);
        let mut max = TileCoord::new(Int::MIN, Int::MIN);

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
