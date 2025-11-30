#![doc = include_str!("LICENSE-hexx.md")]

use core::cmp::{max, min};
use std::{
    fmt::{Display, Formatter},
    ops::{Add, Div, Mul, Neg, Sub},
};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use serde_tuple::{Deserialize_tuple, Serialize_tuple};

use crate::{
    game::coord::layout::TileLayout,
    math::{Float, IVec2, Int, Matrix4, UInt, Vec2, consts},
};

pub mod storage {
    use crate::game::coord::{RadialTileBounds, TileCoord, TileUnit, UnsignedTileUnit};

    #[derive(Debug, Clone)]
    pub struct HexModMap<T> {
        inner: Vec<T>,
        meta: MapMetadata,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    struct MapMetadata {
        bounds: RadialTileBounds,
    }

    /// Converts `self` to [hexmod] coordinates according to the given `range`
    ///
    /// [hexmod]: https://observablehq.com/@sanderevers/hexmod-representation
    #[inline]
    #[must_use]
    #[expect(clippy::cast_possible_wrap)]
    pub const fn to_hexmod_coordinates(coord: TileCoord, range: UnsignedTileUnit) -> UnsignedTileUnit {
        let area = TileCoord::range_count(range) as TileUnit;
        let shift = TileCoord::shift(range) as TileUnit;
        (coord.r + shift * coord.q).rem_euclid(area) as UnsignedTileUnit
    }

    /// Converts [hexmod] to [axial] coordinates according to the given `range`
    ///
    /// # Note
    ///
    /// The resulting coordinate will be wrong if `coord` is not a valid hexmod
    /// value in the given `range`.
    /// `coord` should be lesser or equal to `3 * range * (range + 1) + 1`
    ///
    /// [hexmod]: https://observablehq.com/@sanderevers/hexmod-representation
    /// [axial]: https://www.redblobgames.com/grids/hexagons/#coordinates-axial
    #[expect(clippy::cast_possible_wrap)]
    pub const fn from_hexmod_coordinates(coord: UnsignedTileUnit, range: UnsignedTileUnit) -> TileCoord {
        let shift = TileCoord::shift(range) as TileUnit;
        let range = range as TileUnit;
        let coord = coord as TileUnit;
        let ms = (coord + range) / shift;
        let mcs = (coord + 2 * range) / (shift - 1);
        TileCoord::new(ms * (range + 1) + mcs * -range, coord + ms * (-2 * range - 1) + mcs * (-range - 1))
    }

    impl MapMetadata {
        #[inline]
        const fn new(bounds: RadialTileBounds) -> Self {
            Self {
                bounds,
            }
        }

        /// Converts from hex coordinates to a 1D array index
        #[inline]
        const fn coord_to_idx(&self, coord: TileCoord) -> Option<UnsignedTileUnit> {
            if !self.bounds.contains(coord) {
                return None;
            }

            Some(to_hexmod_coordinates(coord.sub(self.bounds.center), self.bounds.radius))
        }

        /// Converts from a 1D array index back to hex coordinates
        #[inline]
        const fn idx_to_coord(&self, idx: UnsignedTileUnit) -> TileCoord {
            self.bounds.center.add(from_hexmod_coordinates(idx, self.bounds.radius))
        }
    }

    impl<T> HexModMap<T> {
        /// Creates and fills a hexagon shaped map
        ///
        /// # Arguments
        ///
        /// * `center` - The center coordinate of the hexagon
        /// * `radius` - The radius of the map, around `center`
        /// * `values` - Function called for each coordinate in the `radius` to fill the map
        #[must_use]
        pub fn new(center: TileCoord, radius: u32, mut values: impl FnMut(TileCoord) -> T) -> Self {
            let bounds = RadialTileBounds {
                center,
                radius,
            };
            let meta = MapMetadata::new(bounds);

            let hex_count = TileCoord::range_count(bounds.radius);
            // Iterate over all valid hexes in the hexagonal region and fill the map
            let inner: Vec<_> = (0..hex_count)
                .map(|coord| {
                    let hex = center + from_hexmod_coordinates(coord, bounds.radius);
                    values(hex)
                })
                .collect();

            Self {
                inner,
                meta,
            }
        }

        #[inline(always)]
        /// Returns the associated coordinate bounds
        pub const fn bounds(&self) -> RadialTileBounds {
            self.meta.bounds
        }

        #[inline(always)]
        #[must_use]
        pub const fn inner(&self) -> &Vec<T> {
            &self.inner
        }

        #[inline(always)]
        #[must_use]
        pub const fn inner_mut(&mut self) -> &mut Vec<T> {
            &mut self.inner
        }
    }

    impl<T> HexModMap<T> {
        fn get(&self, coord: TileCoord) -> Option<&T> {
            let idx = self.meta.coord_to_idx(coord)?;
            self.inner.get(idx as usize)
        }

        fn get_mut(&mut self, coord: TileCoord) -> Option<&mut T> {
            let idx = self.meta.coord_to_idx(coord)?;
            self.inner.get_mut(idx as usize)
        }
    }
}

pub mod layout {
    use crate::{
        game::coord::{FractCoord, TileCoord},
        math::{Float, Matrix2, Vec2, consts},
    };

    // Mat2 shearing factor
    const FORWARD_SHEAR: Float = consts::SQRT_3 / 2.0;
    const INVERSE_SHEAR: Float = -1.0 / 3.0;

    // Mat2 scale diagonal.
    const FORWARD_SCALE: Vec2 = Vec2::new(consts::SQRT_3, 3.0 / 2.0);
    const INVERSE_SCALE: Vec2 = Vec2::new(consts::SQRT_3 / 3.0, 2.0 / 3.0);

    const FORWARD_MATRIX: Matrix2 = Matrix2 {
        cols: vek::Vec2::new(Vec2::new(FORWARD_SCALE.x, 0.0), Vec2::new(FORWARD_SHEAR, FORWARD_SCALE.y)),
    };
    const INVERSE_MATRIX: Matrix2 = Matrix2 {
        cols: vek::Vec2::new(Vec2::new(INVERSE_SCALE.x, 0.0), Vec2::new(INVERSE_SHEAR, INVERSE_SCALE.y)),
    };

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct TileLayout {
        /// The origin of the hexagonal representation in world/pixel space, usually
        /// [`Vec2::ZERO`]
        pub origin: Vec2,
        /// The size of individual hexagons in world/pixel space. The scale can be
        /// irregular or negative
        pub scale: Vec2,
    }

    impl TileLayout {
        /// Transforms a local hex space vector to world space
        /// by applying the layout `scale` but NOT the origin
        #[inline(always)]
        #[must_use]
        pub(super) fn transform_vector(&self, vector: Vec2) -> Vec2 {
            vector * self.scale
        }

        /// Transforms a local hex point to world space
        /// by applying the layout `scale` and `origin`
        #[inline(always)]
        #[must_use]
        pub(super) fn transform_point(&self, point: Vec2) -> Vec2 {
            self.origin + self.transform_vector(point)
        }

        /// Transforms a world space vector to local hex space
        /// by applying the layout `scale` but NOT the origin
        #[inline(always)]
        #[must_use]
        pub(super) fn inverse_transform_vector(&self, vector: Vec2) -> Vec2 {
            vector / self.scale
        }

        /// Transforms a world pace point to local hex space
        /// by applying the layout `scale` and `origin`
        #[inline(always)]
        #[must_use]
        pub(super) fn inverse_transform_point(&self, point: Vec2) -> Vec2 {
            self.inverse_transform_vector(point - self.origin)
        }
    }

    impl TileLayout {
        #[inline(always)]
        #[must_use]
        /// Computes hexagonal coordinates `hex` into world/pixel coordinates
        pub(super) fn coord_to_world_pos(&self, coord: TileCoord) -> Vec2 {
            self.coord_to_center_aligned_world_pos(coord) + self.origin
        }

        #[inline(always)]
        #[must_use]
        /// Computes hexagonal coordinates `hex` into world/pixel coordinates but
        /// ignoring [`HexLayout::origin`]
        pub(crate) fn coord_to_center_aligned_world_pos(&self, coord: TileCoord) -> Vec2 {
            let p = FORWARD_MATRIX * coord.as_vec2();
            self.transform_vector(p)
        }

        #[inline(always)]
        #[must_use]
        /// Computes fractional hexagonal coordinates `hex` into world/pixel
        /// coordinates
        pub(super) fn fract_coord_to_world_pos(&self, coord: FractCoord) -> Vec2 {
            let p = FORWARD_MATRIX * coord;
            self.transform_point(p)
        }

        #[inline(always)]
        /// Computes world/pixel coordinates `pos` into hexagonal coordinates
        pub(super) fn world_pos_to_coord(&self, pos: Vec2) -> TileCoord {
            let p = self.world_pos_to_fract_coord(pos);
            TileCoord::round(p)
        }

        #[inline(always)]
        #[must_use]
        /// Computes world/pixel coordinates `pos` into fractional hexagonal
        /// coordinates
        pub(super) fn world_pos_to_fract_coord(&self, pos: Vec2) -> FractCoord {
            let point = self.inverse_transform_point(pos);
            INVERSE_MATRIX * point
        }
    }
}

const TILE_GRID_LAYOUT: TileLayout = TileLayout {
    origin: Vec2::new(0.0, 0.0),
    scale: Vec2::new(1.0, 1.0),
};

pub type TileUnit = Int;
pub type UnsignedTileUnit = UInt;

/// A Tile coordinate, in [Offset](https://www.redblobgames.com/grids/hexagons/#coordinates-offset) coordinate system.
pub type OffsetCoord = vek::Vec2<TileUnit>;

/// A Tile coordinate, in [Axial](https://www.redblobgames.com/grids/hexagons/#coordinates-axial) coordinate system.
#[must_use]
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize_tuple, Deserialize_tuple, Pod, Zeroable)]
pub struct TileCoord {
    pub q: TileUnit,
    pub r: TileUnit,
}

/// A fractional Tile coordinate, in the same coordinate system as [`TileCoord`].
pub type FractCoord = Vec2;

impl TileCoord {
    /// Creates a new coordinate `(q, r)`.
    #[inline(always)]
    pub const fn new(q: TileUnit, r: TileUnit) -> Self {
        Self {
            q,
            r,
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn as_vec2(self) -> Vec2 {
        Vec2 {
            x: self.q as Float,
            y: self.r as Float,
        }
    }

    #[inline(always)]
    #[must_use]
    pub fn offset_from_world_pos(pos: Vec2) -> OffsetCoord {
        Self::from_world_pos(pos).to_offset()
    }

    /// Converts from an ['offset coord'](https://www.redblobgames.com/grids/hexagons/#coordinates-offset), in "Even" layout, "Pointy" orientation.
    ///
    /// In the format `[col, row]`.
    #[inline(always)]
    pub const fn from_offset(coord: OffsetCoord) -> Self {
        Self::new(coord.x - TileUnit::midpoint(coord.y, coord.y & 1), coord.y)
    }

    /// Converts to an ['offset coord'](https://www.redblobgames.com/grids/hexagons/#coordinates-offset), in "Even" layout, "Pointy" orientation.
    ///
    /// In the format `[col, row]`.
    #[inline(always)]
    #[must_use]
    pub const fn to_offset(self) -> OffsetCoord {
        OffsetCoord::new(self.q + TileUnit::midpoint(self.r, self.r & 1), self.r)
    }

    #[inline(always)]
    #[must_use]
    pub const fn s(self) -> TileUnit {
        -self.q - self.r
    }

    pub const ZERO: Self = Self::new(0, 0);

    pub const TOP_RIGHT: Self = Self::new(0, 1);
    pub const TOP_LEFT: Self = Self::new(-1, 1);
    pub const LEFT: Self = Self::new(-1, 0);
    pub const BOTTOM_LEFT: Self = Self::new(0, -1);
    pub const BOTTOM_RIGHT: Self = Self::new(1, -1);
    pub const RIGHT: Self = Self::new(1, 0);

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
    #[inline(always)]
    pub const fn add(self, rhs: Self) -> Self {
        Self::new(self.q + rhs.q, self.r + rhs.r)
    }

    #[inline(always)]
    pub const fn sub(self, rhs: Self) -> Self {
        Self::new(self.q - rhs.q, self.r - rhs.r)
    }

    #[inline(always)]
    pub const fn mul(self, rhs: TileUnit) -> Self {
        Self::new(self.q * rhs, self.r * rhs)
    }

    #[inline(always)]
    pub const fn div(self, rhs: TileUnit) -> Self {
        Self::new(self.q / rhs, self.r / rhs)
    }

    #[inline(always)]
    pub const fn neg(self) -> Self {
        Self::new(-self.q, -self.r)
    }

    #[inline(always)]
    pub const fn max(self) -> TileUnit {
        max(self.q, self.r)
    }

    #[inline(always)]
    pub const fn min(self) -> TileUnit {
        min(self.q, self.r)
    }

    /// Creates a list of the neighbors
    #[inline(always)]
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
    #[inline(always)]
    pub const fn cw(self) -> Self {
        Self::new(-self.r, -self.s())
    }

    /// Rotates `self` around `origin` clockwise by `m` turns.
    #[inline(always)]
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
    #[inline(always)]
    pub const fn cw_around(self, center: Self) -> Self {
        self.sub(center).cw().add(center)
    }

    /// Rotates `self` around `center` clockwise by `m` turns.
    #[inline(always)]
    pub const fn rotate_cw_around(self, center: Self, m: u32) -> Self {
        self.sub(center).rotate_cw(m).add(center)
    }

    /// Rotates `self` around `origin` counter-clockwise.
    #[inline(always)]
    pub const fn ccw(self) -> Self {
        Self::new(-self.s(), -self.q)
    }

    /// Rotates `self` around `origin` counter-clockwise by `m` turns.
    #[inline(always)]
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
    #[inline(always)]
    pub const fn ccw_around(self, center: Self) -> Self {
        self.sub(center).ccw().add(center)
    }

    /// Rotates `self` around `center` counter-clockwise by `m` turns.
    #[inline(always)]
    pub const fn rotate_ccw_around(self, center: Self, m: u32) -> Self {
        self.sub(center).rotate_ccw(m).add(center)
    }

    #[inline(always)]
    pub const fn round(coord: FractCoord) -> Self {
        let mut q = coord.x;
        let mut r = coord.y;

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
    #[inline(always)]
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
    #[inline(always)]
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

    #[inline(always)]
    #[must_use]
    /// Counts how many coordinates there are in the given `range`.
    pub const fn range_count(range: UnsignedTileUnit) -> UnsignedTileUnit {
        3 * range * (range + 1) + 1
    }

    /// Shift constant used for [hexmod] operations.
    ///
    /// [hexmod]: https://observablehq.com/@sanderevers/hexmod-representation
    #[inline(always)]
    #[must_use]
    pub(crate) const fn shift(range: UnsignedTileUnit) -> UnsignedTileUnit {
        3 * range + 2
    }

    /// Computes the coordinate of a lower resolution hexagon containing `self`
    /// of a given `radius`.
    /// The lower resolution coordinate can be considered *parent* of
    /// the contained higher resolution coordinates.
    /// The `radius` can be thought of as a *chunk size*, as if the grid was
    /// split in hexagonal chunks of that radius. The returned value are the
    /// coordinates of that chunk, in its own coordinates system.
    ///
    /// See the [source] documentation for more information
    ///
    /// > See also [`Self::to_higher_res`] and [`Self::to_local`]
    ///
    /// [source]: https://observablehq.com/@sanderevers/hexagon-tiling-of-an-hexagonal-grid
    #[expect(clippy::cast_possible_wrap, clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn to_lower_res(self, radius: UnsignedTileUnit) -> Self {
        let [q, r, s] = [self.q, self.r, self.s()];
        let area = Self::range_count(radius) as f32;
        let shift = Self::shift(radius) as TileUnit;
        // TODO f32 and then floor??? integer div???
        let [q, r, s] = [
            ((r + shift * q) as f32 / area).floor(),
            ((s + shift * r) as f32 / area).floor(),
            ((q + shift * s) as f32 / area).floor(),
        ];
        #[rustfmt::skip]
        let [q, r] = [
            ((1.0 + q - r) / 3.0).floor() as TileUnit,
            ((1.0 + r - s) / 3.0).floor() as TileUnit,
        ];
        Self::new(q, r)
    }

    /// Computes the center coordinates of `self` in a higher resolution system
    /// of a given `radius`.
    /// The higher resolution coordinate can be considered as a *child* of
    /// `self` as it is contained by it in a lower resolution coordinates
    /// system. The `radius` can be thought of as a *chunk size*, as if the
    /// grid was split in hexagonal chunks of that radius. The returned
    /// value are the coordinates of the center that chunk, in a higher
    /// resolution coordinates system.
    ///
    /// See the [source] documentation for more information
    ///
    /// > See also [`Self::to_lower_res`] and [`Self::to_local`]
    ///
    /// [source]: https://observablehq.com/@sanderevers/hexagon-tiling-of-an-hexagonal-grid
    #[expect(clippy::cast_possible_wrap)]
    pub const fn to_higher_res(self, radius: UnsignedTileUnit) -> Self {
        let range = radius as TileUnit;
        let [q, r, s] = [self.q, self.r, self.s()];
        Self::new(q * (range + 1) - range * s, r * (range + 1) - range * q)
    }

    /// Computes the local coordinates of `self` in a lower resolution
    /// coordinates system relative to its containing *parent* hexagon
    ///
    ///
    /// See the [source] documentation for more information
    ///
    /// > See also [`Self::to_lower_res`] and [`Self::to_local`]
    ///
    /// [source]: https://observablehq.com/@sanderevers/hexagon-tiling-of-an-hexagonal-grid
    pub fn to_local(self, radius: UnsignedTileUnit) -> Self {
        let upscale = self.to_lower_res(radius);
        let center = upscale.to_higher_res(radius);
        self.sub(center)
    }

    #[inline(always)]
    #[must_use]
    pub fn to_world_pos_with(self, layout: TileLayout) -> Vec2 {
        layout.coord_to_world_pos(self)
    }

    #[inline(always)]
    #[must_use]
    pub fn to_world_pos(self) -> Vec2 {
        self.to_world_pos_with(TILE_GRID_LAYOUT)
    }

    #[inline(always)]
    pub fn from_world_pos_with(pos: Vec2, layout: TileLayout) -> Self {
        layout.world_pos_to_coord(pos)
    }

    #[inline(always)]
    pub fn from_world_pos(pos: Vec2) -> Self {
        Self::from_world_pos_with(pos, TILE_GRID_LAYOUT)
    }

    #[inline(always)]
    #[must_use]
    pub fn fract_coord_to_world_with(coord: FractCoord, layout: TileLayout) -> Vec2 {
        layout.fract_coord_to_world_pos(coord)
    }

    #[inline(always)]
    #[must_use]
    pub fn fract_coord_to_world(coord: FractCoord) -> Vec2 {
        Self::fract_coord_to_world_with(coord, TILE_GRID_LAYOUT)
    }

    #[inline(always)]
    #[must_use]
    pub fn world_to_fract_coord_with(pos: Vec2, layout: TileLayout) -> FractCoord {
        layout.world_pos_to_fract_coord(pos)
    }

    #[inline(always)]
    #[must_use]
    pub fn world_to_fract_coord(pos: Vec2) -> FractCoord {
        Self::world_to_fract_coord_with(pos, TILE_GRID_LAYOUT)
    }

    #[inline(always)]
    #[must_use]
    pub fn lerp_coords(a: TileCoord, b: TileCoord, t: Float) -> FractCoord {
        let a = Vec2::new(a.q as Float, a.r as Float);
        let b = Vec2::new(b.q as Float, b.r as Float);

        FractCoord::lerp(a, b, t)
    }

    #[inline(always)]
    #[must_use]
    pub fn as_translation(self) -> Matrix4 {
        Matrix4::translation_3d(self.to_world_pos())
    }

    #[inline(always)]
    #[must_use]
    pub fn world_pos_to_degrees(coord: Vec2) -> Float {
        let rad = (-coord.y).atan2(coord.x);

        (rad - consts::FRAC_PI_3).rem_euclid(consts::TAU).to_degrees().round()
    }

    #[inline(always)]
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
    #[inline(always)]
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

#[must_use]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct RadialTileBounds {
    pub center: TileCoord,
    pub radius: UnsignedTileUnit,
}

impl RadialTileBounds {
    #[inline(always)]
    #[must_use]
    pub const fn contains(&self, coord: TileCoord) -> bool {
        (coord.sub(self.center)).ulength() <= self.radius
    }
}

impl FromIterator<TileCoord> for RadialTileBounds {
    fn from_iter<T: IntoIterator<Item = TileCoord>>(iter: T) -> Self {
        let mut min = TileCoord::new(TileUnit::MAX, TileUnit::MAX);
        let mut max = TileCoord::new(TileUnit::MIN, TileUnit::MIN);

        for coord in iter {
            min.q = min.q.min(coord.q);
            min.r = min.r.min(coord.r);

            max.q = max.q.max(coord.q);
            max.r = max.r.max(coord.r);
        }

        let center = (min + max) / 2;
        let radius = (center - max).ulength() / 2;
        Self {
            center,
            radius,
        }
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize_tuple)]
pub struct RectTileBounds {
    pub min: TileCoord,
    pub max: TileCoord,
}

impl RectTileBounds {
    #[inline(always)]
    #[must_use]
    pub const fn contains(&self, coord: TileCoord) -> bool {
        (self.min.q <= coord.q && coord.q < self.max.q) && (self.min.r <= coord.r && coord.r < self.max.r)
    }
}

impl FromIterator<TileCoord> for RectTileBounds {
    fn from_iter<T: IntoIterator<Item = TileCoord>>(iter: T) -> Self {
        let mut min = TileCoord::new(TileUnit::MAX, TileUnit::MAX);
        let mut max = TileCoord::new(TileUnit::MIN, TileUnit::MIN);

        for coord in iter {
            min.q = min.q.min(coord.q);
            min.r = min.r.min(coord.r);

            max.q = max.q.max(coord.q);
            max.r = max.r.max(coord.r);
        }

        Self {
            min,
            max,
        }
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct OffsetRectTileBounds {
    pub min_col: TileUnit,
    pub min_row: TileUnit,
    pub max_col: TileUnit,
    pub max_row: TileUnit,
}

impl OffsetRectTileBounds {
    #[inline(always)]
    #[must_use]
    pub const fn contains(&self, coord: TileCoord) -> bool {
        let coord = coord.to_offset();

        (self.min_col <= coord.x && coord.x < self.max_col) && (self.min_row <= coord.y && coord.y < self.max_row)
    }
}

#[must_use]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum TileCoordBounds {
    #[default]
    Empty,
    Radial(RadialTileBounds),
    Rect(RectTileBounds),
    OffsetRect(OffsetRectTileBounds),
}

impl From<RadialTileBounds> for TileCoordBounds {
    fn from(value: RadialTileBounds) -> Self {
        Self::radial(value.center, value.radius)
    }
}

impl From<RectTileBounds> for TileCoordBounds {
    fn from(value: RectTileBounds) -> Self {
        Self::rect(value.min, value.max)
    }
}

impl From<OffsetRectTileBounds> for TileCoordBounds {
    fn from(value: OffsetRectTileBounds) -> Self {
        Self::offset_rect(
            OffsetCoord::new(value.min_col, value.min_row),
            OffsetCoord::new(value.max_col, value.max_row),
        )
    }
}

impl TileCoordBounds {
    #[inline(always)]
    pub const fn radial(center: TileCoord, radius: UnsignedTileUnit) -> Self {
        if radius == 0 {
            return Self::Empty;
        }

        Self::Radial(RadialTileBounds {
            center,
            radius: radius - 1,
        })
    }

    #[inline(always)]
    pub const fn rect(min: TileCoord, max: TileCoord) -> Self {
        if min.q == max.q || min.r == max.r {
            return Self::Empty;
        }

        Self::Rect(RectTileBounds {
            min: TileCoord::new(min.q.min(max.q), min.r.min(max.r)),
            max: TileCoord::new(min.q.max(max.q), min.r.max(max.r)),
        })
    }

    #[inline(always)]
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

    #[inline(always)]
    #[must_use]
    pub const fn contains(&self, coord: TileCoord) -> bool {
        match self {
            TileCoordBounds::Empty => false,
            TileCoordBounds::Radial(v) => v.contains(coord),
            TileCoordBounds::Rect(v) => v.contains(coord),
            TileCoordBounds::OffsetRect(v) => v.contains(coord),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExactSizeCoordIterator {
    bounds: TileCoordBounds,
    count: usize,
    x: TileUnit,
    y: TileUnit,
}

impl ExactSizeCoordIterator {
    #[inline(always)]
    pub fn new(bounds: TileCoordBounds) -> Self {
        match bounds {
            TileCoordBounds::Empty => Self {
                bounds,
                count: 0,
                x: 0,
                y: 0,
            },
            TileCoordBounds::Radial(v) => Self {
                bounds,
                count: TileCoord::range_count(v.radius) as usize,
                x: 0,
                y: 0,
            },
            TileCoordBounds::Rect(v) => {
                let size = v.max - v.min;

                Self {
                    bounds,
                    count: (size.q * size.r) as usize,
                    x: 0,
                    y: 0,
                }
            },
            TileCoordBounds::OffsetRect(v) => Self {
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
                    TileCoordBounds::Empty => {},
                    TileCoordBounds::Radial(v) => {
                        let progress = self.x.unsigned_abs();
                        let ring = self.y.unsigned_abs();

                        debug_assert_eq!(progress, 0);
                        debug_assert_eq!(ring, v.radius + 1);
                    },
                    TileCoordBounds::Rect(v) => {
                        let q = self.x;
                        let r = self.y;

                        debug_assert_eq!(q, 0);
                        debug_assert_eq!(r, v.max.r - v.min.r);
                    },
                    TileCoordBounds::OffsetRect(v) => {
                        let x = self.x;
                        let y = self.y;

                        debug_assert_eq!(x, 0);
                        debug_assert_eq!(y, v.max_row - v.min_row);
                    },
                }
            }
            return None;
        }

        let result;
        match self.bounds {
            TileCoordBounds::Empty => unreachable!(),
            TileCoordBounds::Radial(v) => {
                let mut progress = self.x.unsigned_abs();
                let mut ring = self.y.unsigned_abs();

                #[cfg(debug_assertions)]
                if ring > v.radius {
                    panic!(
                        "ExactSizeCoordIterator exceeded the radius of a TileBounds::Radial. Iterator state: {:?}",
                        self
                    )
                }

                #[allow(clippy::manual_checked_ops)]
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
            },
            TileCoordBounds::Rect(v) => {
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
            },
            TileCoordBounds::OffsetRect(v) => {
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
            },
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

impl IntoIterator for TileCoordBounds {
    type Item = TileCoord;
    type IntoIter = ExactSizeCoordIterator;

    fn into_iter(self) -> Self::IntoIter {
        ExactSizeCoordIterator::new(self)
    }
}
