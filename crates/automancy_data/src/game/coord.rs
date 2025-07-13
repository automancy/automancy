use core::cmp::{max, min};
use std::{
    fmt::{Display, Formatter},
    ops::{Add, Div, Mul, Neg, Sub},
};

use serde::{Deserialize, Serialize};
use serde_tuple::{Deserialize_tuple, Serialize_tuple};

use crate::math::{Float, IVec2, Int, Matrix4, UInt, Vec2, consts};

// TODO: abandon hexx entirely and copy code from their impl, also give credit
const TILE_GRID_LAYOUT: hexx::HexLayout = hexx::HexLayout {
    orientation: hexx::HexOrientation::Pointy,
    origin: hexx::Vec2::ZERO,
    scale: hexx::Vec2::ONE,
};

/// The type that will be used to represent a tile's coordinates.
pub type TileUnit = Int;
pub type UnsignedTileUnit = UInt;

pub type OffsetCoord = vek::Vec2<TileUnit>;

/// A fractional hex coordinate
pub type FractHex = Vec2;

/// Represents a tile's coordinate.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize_tuple, Deserialize_tuple)]
pub struct TileCoord {
    pub q: TileUnit,
    pub r: TileUnit,
}

impl TileCoord {
    /// Creates a new coordinate `(q, r)`.
    #[inline]
    #[must_use]
    pub const fn new(q: TileUnit, r: TileUnit) -> Self {
        Self { q, r }
    }

    #[inline]
    #[must_use]
    pub const fn to_hex(self) -> hexx::Hex {
        hexx::Hex::new(self.q, self.r)
    }

    #[inline]
    #[must_use]
    pub const fn from_hex(hex: hexx::Hex) -> Self {
        Self { q: hex.x, r: hex.y }
    }

    #[inline]
    #[must_use]
    pub fn offset_from_world_pos(pos: Vec2) -> OffsetCoord {
        Self::from_world_pos(pos).to_offset()
    }

    /// Converts from an ['offset coord'](https://www.redblobgames.com/grids/hexagons/#coordinates-offset), in "Even" layout, "Pointy" orientation.
    ///
    /// In the format `[col, row]`.
    #[inline]
    #[must_use]
    pub const fn from_offset(coord: OffsetCoord) -> Self {
        Self::new(coord.x - TileUnit::midpoint(coord.y, coord.y & 1), coord.y)
    }

    /// Converts to an ['offset coord'](https://www.redblobgames.com/grids/hexagons/#coordinates-offset), in "Even" layout, "Pointy" orientation.
    ///
    /// In the format `[col, row]`.
    #[inline]
    #[must_use]
    pub const fn to_offset(self) -> OffsetCoord {
        OffsetCoord::new(self.q + i32::midpoint(self.r, self.r & 1), self.r)
    }

    #[inline]
    #[must_use]
    pub const fn from_dir(dir: hexx::EdgeDirection) -> Self {
        Self::from_hex(dir.into_hex())
    }

    #[inline]
    #[must_use]
    pub const fn s(self) -> TileUnit {
        -self.q - self.r
    }

    pub const ZERO: Self = Self::new(0, 0);

    pub const TOP_RIGHT: Self = Self::from_dir(hexx::EdgeDirection::POINTY_TOP_RIGHT);
    pub const TOP_LEFT: Self = Self::from_dir(hexx::EdgeDirection::POINTY_TOP_LEFT);
    pub const LEFT: Self = Self::from_dir(hexx::EdgeDirection::POINTY_LEFT);
    pub const BOTTOM_LEFT: Self = Self::from_dir(hexx::EdgeDirection::POINTY_BOTTOM_LEFT);
    pub const BOTTOM_RIGHT: Self = Self::from_dir(hexx::EdgeDirection::POINTY_BOTTOM_RIGHT);
    pub const RIGHT: Self = Self::from_dir(hexx::EdgeDirection::POINTY_RIGHT);

    pub const EDGES: [Self; 6] = [
        Self::TOP_RIGHT,
        Self::TOP_LEFT,
        Self::LEFT,
        Self::BOTTOM_LEFT,
        Self::BOTTOM_RIGHT,
        Self::RIGHT,
    ];
}

impl TileCoord {
    #[inline]
    #[must_use]
    pub const fn add(self, rhs: Self) -> Self {
        Self::new(self.q + rhs.q, self.r + rhs.r)
    }

    #[inline]
    #[must_use]
    pub const fn sub(self, rhs: Self) -> Self {
        Self::new(self.q - rhs.q, self.r - rhs.r)
    }

    #[inline]
    #[must_use]
    pub const fn mul(self, rhs: TileUnit) -> Self {
        Self::new(self.q * rhs, self.r * rhs)
    }

    #[inline]
    #[must_use]
    pub const fn div(self, rhs: TileUnit) -> Self {
        Self::new(self.q / rhs, self.r / rhs)
    }

    #[inline]
    #[must_use]
    pub const fn neg(self) -> Self {
        Self::new(-self.q, -self.r)
    }

    #[inline]
    #[must_use]
    pub const fn max(self) -> TileUnit {
        max(self.q, self.r)
    }

    #[inline]
    #[must_use]
    pub const fn min(self) -> TileUnit {
        min(self.q, self.r)
    }

    /// Creates a list of the neighbors
    #[inline]
    #[must_use]
    pub const fn neighbors(self) -> [Self; 6] {
        [
            self.add(Self::EDGES[0]),
            self.add(Self::EDGES[1]),
            self.add(Self::EDGES[2]),
            self.add(Self::EDGES[3]),
            self.add(Self::EDGES[4]),
            self.add(Self::EDGES[5]),
        ]
    }

    /// Rotates `self` around `origin` clockwise.
    #[inline]
    #[must_use]
    pub const fn cw(self) -> Self {
        Self::new(-self.r, -self.s())
    }

    /// Rotates `self` around `origin` clockwise by `m` turns.
    #[inline]
    #[must_use]
    pub const fn rotate_cw(self, m: u32) -> Self {
        match m % 6 {
            1 => self.cw(),
            2 => self.cw().cw(),
            3 => self.neg(),
            4 => self.ccw().ccw(),
            5 => self.ccw(),
            _ => self,
        }
    }

    /// Rotates `self` around `center` clockwise.
    #[inline]
    #[must_use]
    pub const fn cw_around(self, center: Self) -> Self {
        self.sub(center).cw().add(center)
    }

    /// Rotates `self` around `center` clockwise by `m` turns.
    #[inline]
    #[must_use]
    pub const fn rotate_cw_around(self, center: Self, m: u32) -> Self {
        self.sub(center).rotate_cw(m).add(center)
    }

    /// Rotates `self` around `origin` counter-clockwise.
    #[inline]
    #[must_use]
    pub const fn ccw(self) -> Self {
        Self::new(-self.s(), -self.q)
    }

    /// Rotates `self` around `origin` counter-clockwise by `m` turns.
    #[inline]
    #[must_use]
    pub const fn rotate_ccw(self, m: u32) -> Self {
        match m % 6 {
            1 => self.ccw(),
            2 => self.ccw().ccw(),
            3 => self.neg(),
            4 => self.cw().cw(),
            5 => self.cw(),
            _ => self,
        }
    }

    /// Rotates `self` around `center` counter-clockwise.
    #[inline]
    #[must_use]
    pub const fn ccw_around(self, center: Self) -> Self {
        self.sub(center).ccw().add(center)
    }

    /// Rotates `self` around `center` counter-clockwise by `m` turns.
    #[inline]
    #[must_use]
    pub const fn rotate_ccw_around(self, center: Self, m: u32) -> Self {
        self.sub(center).rotate_ccw(m).add(center)
    }

    #[inline]
    #[must_use]
    pub const fn round(fract: FractHex) -> Self {
        let mut q = fract.x;
        let mut r = fract.y;

        let [mut q_r, mut r_r] = [q.round(), r.round()];
        q -= q_r;
        r -= r_r;
        if q.abs() >= r.abs() {
            q_r += 0.5_f32.mul_add(r, q).round();
        } else {
            r_r += 0.5_f32.mul_add(q, r).round();
        }

        Self::new(q_r as TileUnit, r_r as TileUnit)
    }

    /// Computes coordinates length as a signed integer.
    ///
    /// See [`Self::ulength`] for the unsigned version
    #[inline]
    #[must_use]
    pub const fn length(self) -> TileUnit {
        let [q, r, s] = [self.q.abs(), self.r.abs(), self.s().abs()];
        if q >= r && q >= s {
            q
        } else if r >= q && r >= s {
            r
        } else {
            s
        }
    }

    /// Computes coordinates length as an unsigned integer.
    ///
    /// See [`Self::length`] for the signed version
    #[inline]
    #[must_use]
    pub const fn ulength(self) -> UnsignedTileUnit {
        let [q, r, s] = [self.q.unsigned_abs(), self.r.unsigned_abs(), self.s().unsigned_abs()];

        if q >= r && q >= s {
            q
        } else if r >= q && r >= s {
            r
        } else {
            s
        }
    }

    #[inline]
    #[must_use]
    pub fn to_world_pos(self) -> Vec2 {
        let p = TILE_GRID_LAYOUT.hex_to_world_pos(self.to_hex());

        Vec2::new(p.x, p.y)
    }

    #[inline]
    #[must_use]
    pub fn from_world_pos(pos: Vec2) -> Self {
        Self::from_hex(TILE_GRID_LAYOUT.world_pos_to_hex(hexx::Vec2::new(pos.x, pos.y)))
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
        let a = Vec2::new(a.q as Float, a.r as Float);
        let b = Vec2::new(b.q as Float, b.r as Float);

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

        (rad - consts::FRAC_PI_3).rem_euclid(consts::TAU).to_degrees().round()
    }

    #[inline]
    #[must_use]
    pub fn as_degrees(self) -> Float {
        match self {
            Self::TOP_RIGHT => 0.0,
            Self::TOP_LEFT => 60.0,
            Self::LEFT => 120.0,
            Self::BOTTOM_LEFT => 180.0,
            Self::BOTTOM_RIGHT => 240.0,
            Self::RIGHT => 300.0,
            _ => Self::world_pos_to_degrees(self.to_world_pos()),
        }
    }

    /// Creates a minimal string representation of the coordinate.
    #[inline]
    #[must_use]
    pub fn to_minimal_string(self) -> String {
        format!("{},{}", self.q, self.r)
    }
}

impl Display for TileCoord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{}, {}]", self.q, self.r))
    }
}

impl Add for TileCoord {
    type Output = TileCoord;

    fn add(self, rhs: TileCoord) -> Self::Output {
        self.add(rhs)
    }
}

impl Sub for TileCoord {
    type Output = TileCoord;

    fn sub(self, rhs: TileCoord) -> Self::Output {
        self.sub(rhs)
    }
}

impl Mul<TileUnit> for TileCoord {
    type Output = TileCoord;

    fn mul(self, rhs: TileUnit) -> Self::Output {
        self.mul(rhs)
    }
}

impl Mul<UnsignedTileUnit> for TileCoord {
    type Output = TileCoord;

    fn mul(self, rhs: UnsignedTileUnit) -> Self::Output {
        self.mul(rhs.cast_signed())
    }
}

impl Mul<TileCoord> for TileUnit {
    type Output = TileCoord;

    fn mul(self, rhs: TileCoord) -> Self::Output {
        rhs.mul(self)
    }
}

impl Div<TileUnit> for TileCoord {
    type Output = TileCoord;

    fn div(self, rhs: TileUnit) -> Self::Output {
        self.div(rhs)
    }
}

impl Neg for TileCoord {
    type Output = TileCoord;

    fn neg(self) -> Self::Output {
        self.neg()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize_tuple)]
pub struct RadialTileBounds {
    center: TileCoord,
    radius: UnsignedTileUnit,
}

impl RadialTileBounds {
    #[inline]
    #[must_use]
    pub const fn contains(&self, coord: TileCoord) -> bool {
        (coord.sub(self.center)).ulength() <= self.radius
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize_tuple)]
pub struct RectTileBounds {
    min: TileCoord,
    max: TileCoord,
}

impl RectTileBounds {
    #[inline]
    #[must_use]
    pub const fn contains(&self, coord: TileCoord) -> bool {
        (self.min.q <= coord.q && coord.q < self.max.q) && (self.min.r <= coord.r && coord.r < self.max.r)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize_tuple)]
pub struct OffsetRectTileBounds {
    min_col: TileUnit,
    min_row: TileUnit,
    max_col: TileUnit,
    max_row: TileUnit,
}

impl OffsetRectTileBounds {
    #[inline]
    #[must_use]
    pub const fn contains(&self, coord: TileCoord) -> bool {
        let coord = coord.to_offset();

        (self.min_col <= coord.x && coord.x < self.max_col) && (self.min_row <= coord.y && coord.y < self.max_row)
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum TileBounds {
    #[default]
    Empty,
    Radial(RadialTileBounds),
    Rect(RectTileBounds),
    OffsetRect(OffsetRectTileBounds),
}

impl From<RadialTileBounds> for TileBounds {
    fn from(value: RadialTileBounds) -> Self {
        Self::radial(value.center, value.radius)
    }
}

impl From<RectTileBounds> for TileBounds {
    fn from(value: RectTileBounds) -> Self {
        Self::rect(value.min, value.max)
    }
}

impl From<OffsetRectTileBounds> for TileBounds {
    fn from(value: OffsetRectTileBounds) -> Self {
        Self::offset_rect(
            OffsetCoord::new(value.min_col, value.min_row),
            OffsetCoord::new(value.max_col, value.max_row),
        )
    }
}

impl TileBounds {
    #[inline]
    #[must_use]
    pub const fn radial(center: TileCoord, radius: UnsignedTileUnit) -> Self {
        if radius == 0 {
            return Self::Empty;
        }

        Self::Radial(RadialTileBounds { center, radius: radius - 1 })
    }

    #[inline]
    #[must_use]
    pub const fn rect(min: TileCoord, max: TileCoord) -> Self {
        if min.q == max.q || min.r == max.r {
            return Self::Empty;
        }

        Self::Rect(RectTileBounds {
            min: TileCoord::new(min.q.min(max.q), min.r.min(max.r)),
            max: TileCoord::new(min.q.max(max.q), min.r.max(max.r)),
        })
    }

    #[inline]
    #[must_use]
    pub const fn offset_rect(min: OffsetCoord, max: OffsetCoord) -> Self {
        if min.x == max.x || min.y == max.y {
            return Self::Empty;
        }

        Self::OffsetRect(OffsetRectTileBounds {
            min_col: min.x.min(max.x),
            min_row: min.y.min(max.y),
            max_col: min.x.max(max.x),
            max_row: min.y.max(max.y),
        })
    }

    #[inline]
    #[must_use]
    pub const fn contains(&self, coord: TileCoord) -> bool {
        match self {
            TileBounds::Empty => false,
            TileBounds::Radial(v) => v.contains(coord),
            TileBounds::Rect(v) => v.contains(coord),
            TileBounds::OffsetRect(v) => v.contains(coord),
        }
    }
}

impl FromIterator<TileCoord> for TileBounds {
    fn from_iter<T: IntoIterator<Item = TileCoord>>(iter: T) -> Self {
        let mut min = TileCoord::new(TileUnit::MAX, TileUnit::MAX);
        let mut max = TileCoord::new(TileUnit::MIN, TileUnit::MIN);

        for coord in iter {
            min.q = min.q.min(coord.q);
            min.r = min.r.min(coord.r);

            max.q = max.q.max(coord.q);
            max.r = max.r.max(coord.r);
        }
        Self::rect(min, max)
    }
}

#[derive(Debug, Clone)]
pub struct ExactSizeCoordIterator {
    bounds: TileBounds,
    count: usize,
    x: TileUnit,
    y: TileUnit,
}

impl ExactSizeCoordIterator {
    #[inline]
    #[must_use]
    pub fn new(bounds: TileBounds) -> Self {
        match bounds {
            TileBounds::Empty => Self {
                bounds,
                count: 0,
                x: 0,
                y: 0,
            },
            TileBounds::Radial(v) => Self {
                bounds,
                // formula copied from https://www.redblobgames.com/grids/hexagons/#rings-spiral
                count: (1 + 3 * v.radius * (v.radius + 1)) as usize,
                x: 0,
                y: 0,
            },
            TileBounds::Rect(v) => {
                let size = v.max - v.min;

                Self {
                    bounds,
                    count: (size.q * size.r) as usize,
                    x: 0,
                    y: 0,
                }
            }
            TileBounds::OffsetRect(v) => Self {
                bounds,
                count: ((v.max_col - v.min_col) * (v.max_row - v.min_row)) as usize,
                x: 0,
                y: 0,
            },
        }
    }
}

impl Iterator for ExactSizeCoordIterator {
    type Item = TileCoord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            #[cfg(debug_assertions)]
            {
                match self.bounds {
                    TileBounds::Empty => {}
                    TileBounds::Radial(v) => {
                        let progress = self.x.unsigned_abs();
                        let ring = self.y.unsigned_abs();

                        debug_assert_eq!(progress, 0);
                        debug_assert_eq!(ring, v.radius + 1);
                    }
                    TileBounds::Rect(v) => {
                        let q = self.x;
                        let r = self.y;

                        debug_assert_eq!(q, 0);
                        debug_assert_eq!(r, v.max.r - v.min.r);
                    }
                    TileBounds::OffsetRect(v) => {
                        let x = self.x;
                        let y = self.y;

                        debug_assert_eq!(x, 0);
                        debug_assert_eq!(y, v.max_row - v.min_row);
                    }
                }
            }
            return None;
        }

        let result;
        match self.bounds {
            TileBounds::Empty => unreachable!(),
            TileBounds::Radial(v) => {
                let mut progress = self.x.unsigned_abs();
                let mut ring = self.y.unsigned_abs();

                #[cfg(debug_assertions)]
                if ring > v.radius {
                    panic!(
                        "ExactSizeCoordIterator exceeded the radius of a TileBounds::Radial. Iterator state: {:?}",
                        self
                    )
                }

                if ring == 0 {
                    result = Some(v.center);
                    progress = 0;
                    ring = 1;
                } else {
                    let part = (progress / ring) as usize;
                    let curr_dir = TileCoord::EDGES[part % 6];
                    let next_dir = TileCoord::EDGES[(part + 2) % 6];

                    result = Some(v.center + curr_dir * ring + next_dir * (progress % ring));
                    progress += 1;

                    if progress == ring * 6 {
                        progress = 0;
                        ring += 1;
                    }
                }

                self.x = progress.cast_signed();
                self.y = ring.cast_signed();
            }
            TileBounds::Rect(v) => {
                let mut q = self.x;
                let mut r = self.y;
                let size = v.max - v.min;

                #[cfg(debug_assertions)]
                if r >= size.r {
                    panic!(
                        "ExactSizeCoordIterator exceeded the max coord of a TileBounds::Rect. Iterator state: {:?}",
                        self
                    )
                }

                result = Some(v.min + TileCoord::new(q, r));
                q += 1;
                if q == size.q {
                    q = 0;
                    r += 1;
                }

                self.x = q;
                self.y = r;
            }
            TileBounds::OffsetRect(v) => {
                let mut x = self.x;
                let mut y = self.y;

                let size = IVec2::new(v.max_col - v.min_col, v.max_row - v.min_row);

                result = Some(TileCoord::from_offset(OffsetCoord::new(v.min_col + x, v.min_row + y)));
                x += 1;
                if x == size.x {
                    x = 0;
                    y += 1;
                }

                self.x = x;
                self.y = y;
            }
        }

        if result.is_none() {
            panic!(
                "ExactSizeCoordIterator finished iteration while count is still positive. Iterator state: {:?}",
                self
            )
        }

        self.count -= 1;
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
