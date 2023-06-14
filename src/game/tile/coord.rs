use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, Mul, Neg, RangeInclusive, Sub};

use hexagon_tiles::fractional::FractionalHex;
use hexagon_tiles::hex::{hex, Hex};
use hexagon_tiles::traits::{HexDirection, HexMath, HexRound};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::util::cg::Double;

/// The type of number that will be stored in a tile's coordinates. Should probably be a signed integer.
pub type TileUnit = i32;

/// The basic Hex Grid coordinate.
pub type TileHex = Hex<TileUnit>;

/// Represents a tile's position.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TileCoord(TileHex);

/// Represents a chunk's position.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ChunkCoord(TileHex);

/// Copied from https://youtu.be/Zz296fdB8rc
impl From<TileCoord> for ChunkCoord {
    fn from(c: TileCoord) -> Self {
        let qm = c.q() * CHUNK_SIZE;
        let rm = c.r() * CHUNK_SIZE;
        let q = ((2 * qm) + c.q() + (rm + c.r())) as Double / CHUNK_ARIA;
        let r = (rm - qm - c.q()) as Double / CHUNK_ARIA;
        let hex = FractionalHex::new(q, r);

        Self(hex.round())
    }
}

impl From<ChunkCoord> for TileCoord {
    fn from(c: ChunkCoord) -> Self {
        Self::new(
            c.q() * CHUNK_SIZE - (CHUNK_SIZE + 1) * c.r(),
            c.q() * (CHUNK_SIZE + 1) + c.r() * (2 * CHUNK_SIZE + 1),
        )
    }
}

// The size of a chunk
pub const CHUNK_SIZE: TileUnit = 16;

pub const CHUNK_SIZE_SQUARED: TileUnit = CHUNK_SIZE * CHUNK_SIZE;

pub const CHUNK_ARIA: Double = (3 * CHUNK_SIZE_SQUARED + 3 * CHUNK_SIZE + 1) as Double;

impl TileCoord {
    /// Shorthand for the tile at position (0, 0, 0).
    pub const ZERO: Self = Self(hex(0, 0, 0));
    /// Creates a new coordinate from a q and an r component, at the position (q, r, -q - r).
    pub fn new(q: TileUnit, r: TileUnit) -> Self {
        Self(Hex::new(q, r))
    }
}

impl TileCoord {
    /// Gets the q component of the coordinate.
    pub fn q(self) -> TileUnit {
        self.0.q()
    }
    /// Gets the r component of the coordinate.
    pub fn r(self) -> TileUnit {
        self.0.r()
    }
    /// Gets the s component of the coordinate.
    pub fn s(self) -> TileUnit {
        self.0.s()
    }

    /// Creates a minimal string of the coordinate.
    pub fn to_minimal_string(self) -> String {
        format!("{},{}", self.q(), self.r())
    }

    /// Constant. Represents the adjacent coord to the top right. Ordinal of 2.
    pub const TOP_RIGHT: Self = Self(TileHex::NEIGHBORS[2]);
    /// Constant. Represents the adjacent coord to the right. Ordinal of 3.
    pub const RIGHT: Self = Self(TileHex::NEIGHBORS[3]);
    /// Constant. Represents the adjacent coord to the bottom right. Ordinal of 4.
    pub const BOTTOM_RIGHT: Self = Self(TileHex::NEIGHBORS[4]);
    /// Constant. Represents the adjacent coord to the bottom left. Ordinal of 5.
    pub const BOTTOM_LEFT: Self = Self(TileHex::NEIGHBORS[5]);
    /// Constant. Represents the adjacent coord to the left. Ordinal of 0.
    pub const LEFT: Self = Self(TileHex::NEIGHBORS[0]);
    /// Constant. Represents the adjacent coord to the top left. Ordinal of 1.
    pub const TOP_LEFT: Self = Self(TileHex::NEIGHBORS[1]);

    /// Constant. Represents the diagonal coord to the top right. Ordinal of 2.
    pub const DIAG_TOP: Self = Self(TileHex::DIAGONALS[1]);
    /// Constant. Represents the diagonal coord to the right. Ordinal of 3.
    pub const DIAG_TOP_RIGHT: Self = Self(TileHex::DIAGONALS[0]);
    /// Constant. Represents the diagonal coord to the bottom right. Ordinal of 4.
    pub const DIAG_BOTTOM_RIGHT: Self = Self(TileHex::DIAGONALS[5]);
    /// Constant. Represents the diagonal coord to the bottom left. Ordinal of 5.
    pub const DIAG_BOTTOM: Self = Self(TileHex::DIAGONALS[4]);
    /// Constant. Represents the diagonal coord to the left. Ordinal of 0.
    pub const DIAG_BOTTOM_LEFT: Self = Self(TileHex::DIAGONALS[3]);
    /// Constant. Represents the diagonal coord to the top left. Ordinal of 1.
    pub const DIAG_TOP_LEFT: Self = Self(TileHex::DIAGONALS[2]);

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

    /// Creates a list of the diagonals
    pub fn diagonals(self) -> [Self; 6] {
        [
            self + Self::DIAG_TOP,
            self + Self::DIAG_TOP_RIGHT,
            self + Self::DIAG_BOTTOM_RIGHT,
            self + Self::DIAG_BOTTOM,
            self + Self::DIAG_BOTTOM_LEFT,
            self + Self::DIAG_TOP_LEFT,
        ]
    }
}

impl ChunkCoord {
    /// Gets the q component of the coordinate.
    pub fn q(self) -> TileUnit {
        self.0.q()
    }
    /// Gets the r component of the coordinate.
    pub fn r(self) -> TileUnit {
        self.0.r()
    }
    /// Gets the s component of the coordinate.
    pub fn s(self) -> TileUnit {
        self.0.s()
    }

    /// Creates a minimal string of the coordinate.
    pub fn to_minimal_string(self) -> String {
        format!("{},{}", self.q(), self.r())
    }

    /// Constant. Represents the adjacent coord to the top right. Ordinal of 2.
    pub const TOP_RIGHT: Self = Self(TileHex::NEIGHBORS[2]);
    /// Constant. Represents the adjacent coord to the right. Ordinal of 3.
    pub const RIGHT: Self = Self(TileHex::NEIGHBORS[3]);
    /// Constant. Represents the adjacent coord to the bottom right. Ordinal of 4.
    pub const BOTTOM_RIGHT: Self = Self(TileHex::NEIGHBORS[4]);
    /// Constant. Represents the adjacent coord to the bottom left. Ordinal of 5.
    pub const BOTTOM_LEFT: Self = Self(TileHex::NEIGHBORS[5]);
    /// Constant. Represents the adjacent coord to the left. Ordinal of 0.
    pub const LEFT: Self = Self(TileHex::NEIGHBORS[0]);
    /// Constant. Represents the adjacent coord to the top left. Ordinal of 1.
    pub const TOP_LEFT: Self = Self(TileHex::NEIGHBORS[1]);

    /// Constant. Represents the diagonal coord to the top right. Ordinal of 2.
    pub const DIAG_TOP: Self = Self(TileHex::DIAGONALS[1]);
    /// Constant. Represents the diagonal coord to the right. Ordinal of 3.
    pub const DIAG_TOP_RIGHT: Self = Self(TileHex::DIAGONALS[0]);
    /// Constant. Represents the diagonal coord to the bottom right. Ordinal of 4.
    pub const DIAG_BOTTOM_RIGHT: Self = Self(TileHex::DIAGONALS[5]);
    /// Constant. Represents the diagonal coord to the bottom left. Ordinal of 5.
    pub const DIAG_BOTTOM: Self = Self(TileHex::DIAGONALS[4]);
    /// Constant. Represents the diagonal coord to the left. Ordinal of 0.
    pub const DIAG_BOTTOM_LEFT: Self = Self(TileHex::DIAGONALS[3]);
    /// Constant. Represents the diagonal coord to the top left. Ordinal of 1.
    pub const DIAG_TOP_LEFT: Self = Self(TileHex::DIAGONALS[2]);

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

    /// Creates a list of the diagonals
    pub fn diagonals(self) -> [Self; 6] {
        [
            self + Self::DIAG_TOP,
            self + Self::DIAG_TOP_RIGHT,
            self + Self::DIAG_BOTTOM_RIGHT,
            self + Self::DIAG_BOTTOM,
            self + Self::DIAG_BOTTOM_LEFT,
            self + Self::DIAG_TOP_LEFT,
        ]
    }
}

impl ChunkCoord {
    pub fn iter(&self) -> impl Iterator<Item = TileCoord> {
        let center: TileCoord = (*self).into();
        let center = center.0;

        HexRangeIterator::new(CHUNK_SIZE).map(move |hex| TileCoord(hex + center))
    }
}

pub struct HexRangeIterator {
    q: RangeInclusive<TileUnit>,
    r: RangeInclusive<TileUnit>,
    current_q: TileUnit,
    size: TileUnit,
}

impl HexRangeIterator {
    pub fn new(range: TileUnit) -> Self {
        Self {
            q: -range + 1..=range,
            r: 0..=range,
            current_q: -range,
            size: range,
        }
    }
}

impl Iterator for HexRangeIterator {
    type Item = Hex<TileUnit>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.r.next() {
            None => match self.q.next() {
                Some(q) => {
                    self.current_q = q;
                    self.r = (-self.size).max(-q - self.size)..=(self.size).min(-q + self.size);

                    if let Some(r) = self.r.next() {
                        Some(Hex::new(self.current_q, r))
                    } else {
                        None
                    }
                }
                None => None,
            },
            Some(r) => Some(Hex::new(self.current_q, r)),
        }
    }
}

impl Display for TileCoord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{}, {}]", self.0.q(), self.0.r()))
    }
}

impl Serialize for TileCoord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.0.q())?;
        tuple.serialize_element(&self.0.r())?;
        tuple.end()
    }
}

struct TileCoordVisitor;

impl<'de> Visitor<'de> for TileCoordVisitor {
    type Value = TileCoord;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let q: TileUnit = seq.next_element()?.unwrap();
        let r: TileUnit = seq.next_element()?.unwrap();

        Ok(TileCoord::new(q, r))
    }
}

impl<'de> Deserialize<'de> for TileCoord
where
    Self: Sized,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, TileCoordVisitor)
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
    /// Gets the distance between two tiles.
    pub fn distance(self, other: TileCoord) -> TileUnit {
        self.0.distance(other.0)
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

/// ChunkCoord math

impl Add for ChunkCoord {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for ChunkCoord {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<TileUnit> for ChunkCoord {
    type Output = Self;

    fn mul(self, rhs: TileUnit) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<TileUnit> for ChunkCoord {
    type Output = Self;

    fn div(self, rhs: TileUnit) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Neg for ChunkCoord {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
