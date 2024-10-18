use crate::math::{Matrix4, FAR, HEX_GRID_LAYOUT};
use hexx::{EdgeDirection, Hex, HexBounds};
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::fmt::{Display, Formatter};
use std::ops::{Add, Deref, Div, Mul, Neg, Sub};

/// The type of number that will be stored in a tile's coordinates. Should probably be a signed integer.
pub type TileUnit = i32;

/// The basic Hex Grid coordinate.
pub type TileHex = Hex;

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
    pub const ZERO: Self = Self(Hex::new(0, 0));

    /// Creates a new coordinate from a q and an r component, at the position (q, r, -q - r).
    pub const fn new(q: TileUnit, r: TileUnit) -> Self {
        Self(TileHex::new(q, r))
    }
}

impl TileCoord {
    /// Creates a minimal string of the coordinate.
    pub fn to_minimal_string(self) -> String {
        format!("{},{}", self.x, self.y)
    }

    pub const TOP_RIGHT: Self = Self(EdgeDirection::POINTY_TOP_RIGHT.into_hex());
    pub const RIGHT: Self = Self(EdgeDirection::POINTY_RIGHT.into_hex());
    pub const BOTTOM_RIGHT: Self = Self(EdgeDirection::POINTY_BOTTOM_RIGHT.into_hex());
    pub const BOTTOM_LEFT: Self = Self(EdgeDirection::POINTY_BOTTOM_LEFT.into_hex());
    pub const LEFT: Self = Self(EdgeDirection::POINTY_LEFT.into_hex());
    pub const TOP_LEFT: Self = Self(EdgeDirection::POINTY_TOP_LEFT.into_hex());

    /// Creates a list of the neighbors
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

    pub fn as_translation(self) -> Matrix4 {
        let p = HEX_GRID_LAYOUT.hex_to_world_pos(self.0);

        Matrix4::from_translation(p.extend(FAR))
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
    Hex(HexBounds),
}

impl TileBounds {
    #[inline]
    #[must_use]
    pub const fn new(center: TileCoord, radius: u32) -> Self {
        Self::Hex(HexBounds {
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
            TileBounds::Empty => TileCoord::ZERO,
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

impl From<HexBounds> for TileBounds {
    fn from(value: HexBounds) -> Self {
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
