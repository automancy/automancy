use std::f64::consts::PI;
use std::ops::{Div, Sub};

use automancy_defs::cg::{matrix, DPoint2, DPoint3, Double};
use automancy_defs::cgmath::{point2, point3, vec2, EuclideanSpace};
use automancy_defs::coord::{TileCoord, TileUnit};
use automancy_defs::hexagon_tiles::fractional::FractionalHex;
use automancy_defs::hexagon_tiles::layout::{hex_to_pixel, pixel_to_hex};
use automancy_defs::hexagon_tiles::point::{point, Point};
use automancy_defs::rendering::HEX_GRID_LAYOUT;

use crate::camera::FAR;

/// Gets the hex position being pointed at.
#[inline]
pub fn main_pos_to_hex(
    width: Double,
    height: Double,
    camera_pos: DPoint3,
    main_pos: DPoint2,
) -> FractionalHex<Double> {
    let p = screen_to_world(width, height, main_pos, camera_pos.z);
    let p = p + camera_pos.to_vec();

    let p = point(p.x, p.y);

    pixel_to_hex(HEX_GRID_LAYOUT, p)
}

/// Converts screen space coordinates into normalized coordinates.
#[inline]
pub fn screen_to_normalized(width: Double, height: Double, c: DPoint2) -> DPoint2 {
    let size = vec2(width, height) / 2.0;

    let c = vec2(c.x, c.y);
    let c = c.zip(size, Sub::sub);
    let c = c.zip(size, Div::div);

    point2(c.x, c.y)
}

/// Converts screen coordinates to world coordinates.
#[inline]
pub fn screen_to_world(width: Double, height: Double, c: DPoint2, camera_z: Double) -> DPoint3 {
    let c = screen_to_normalized(width, height, c);

    normalized_to_world(width, height, c, camera_z)
}

/// Converts normalized screen coordinates to world coordinates.
#[inline]
pub fn normalized_to_world(width: Double, height: Double, p: DPoint2, z: Double) -> DPoint3 {
    let aspect = width / height;

    let matrix = matrix(point3(0.0, 0.0, z), aspect, PI);

    let p = p.to_vec();
    let p = matrix * p.extend(FAR).extend(1.0);
    let p = p.truncate() * p.w;

    let aspect_squared = aspect.powi(2);

    point3(p.x * aspect_squared, p.y, p.z)
}

/// Converts hex coordinates to normalized screen coordinates.
#[inline]
pub fn hex_to_normalized(
    width: Double,
    height: Double,
    camera_pos: DPoint3,
    hex: TileCoord,
) -> (DPoint2, Double) {
    let Point { x, y } = hex_to_pixel(HEX_GRID_LAYOUT, hex.into());

    let aspect = width / height;

    let matrix = matrix(camera_pos, aspect, PI);

    let p = vec2(x, y);
    let p = matrix * p.extend(FAR).extend(1.0);
    let w = p.w;
    let p = p.truncate() / w;

    (point2(p.x, p.y), w)
}

#[inline]
pub fn is_in_culling_range(
    center: TileCoord,
    other: TileCoord,
    culling_range: (TileUnit, TileUnit),
) -> bool {
    center.distance(other) < culling_range.0 && center.distance(other) < culling_range.1
}
