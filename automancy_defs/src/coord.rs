use std::fmt::{Display, Formatter};
use std::ops::{Add, Deref, Div, Mul, Neg, Sub};

use hexx::Hex;
use serde::{Deserialize, Serialize};

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

impl TileCoord {
    /// Shorthand for the tile at position (0, 0).
    pub const ZERO: Self = Self(Hex::new(0, 0));

    /// Creates a new coordinate from a q and an r component, at the position (q, r, -q - r).
    pub fn new(q: TileUnit, r: TileUnit) -> Self {
        Self(TileHex::new(q, r))
    }
}

impl TileCoord {
    /// Creates a minimal string of the coordinate.
    pub fn to_minimal_string(self) -> String {
        format!("{},{}", self.x, self.y)
    }

    pub const TOP_RIGHT: Self = Self(TileHex::NEIGHBORS_COORDS[3]);
    pub const RIGHT: Self = Self(TileHex::NEIGHBORS_COORDS[2]);
    pub const BOTTOM_RIGHT: Self = Self(TileHex::NEIGHBORS_COORDS[1]);
    pub const BOTTOM_LEFT: Self = Self(TileHex::NEIGHBORS_COORDS[0]);
    pub const LEFT: Self = Self(TileHex::NEIGHBORS_COORDS[5]);
    pub const TOP_LEFT: Self = Self(TileHex::NEIGHBORS_COORDS[4]);

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
}

impl Display for TileCoord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{}, {}]", self.0.x, self.0.y))
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

/// TileCoord math

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
