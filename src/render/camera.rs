use std::f64::consts::PI;
use std::ops::{Div, Sub};

use cgmath::{point2, point3, vec2, EuclideanSpace, Zero};
use hexagon_tiles::fractional::FractionalHex;
use hexagon_tiles::layout::{hex_to_pixel, pixel_to_hex};
use hexagon_tiles::point::{point, Point};
use hexagon_tiles::traits::HexRound;
use num::clamp;

use crate::game::input::InputState;
use crate::game::tile::coord::TileCoord;
use crate::render::data::HEX_LAYOUT;
use crate::util::cg::{matrix, DPoint2, DPoint3, DVector2, Double};

pub const FAR: Double = 0.0;

/// Returns if the camera is at its maximum height.
pub fn is_at_max_height(pos: DPoint3) -> bool {
    (1.0 - pos.z).abs() <= 0.001
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pos: DPoint3,
    move_vel: DVector2,
    scroll_vel: Double,

    pub pointing_at: TileCoord,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: point3(0.0, 0.0, 1.0),
            move_vel: vec2(0.0, 0.0),
            scroll_vel: 0.0,

            pointing_at: TileCoord::new(0, 0),
        }
    }
}

impl Camera {
    /// Returns if the camera is at its maximum (most zoomed out) height.
    pub fn is_at_max_height(&self) -> bool {
        is_at_max_height(self.get_pos())
    }

    /// Returns the position of the camera.
    pub fn get_pos(&self) -> DPoint3 {
        let DPoint3 { x, y, z } = self.pos;

        point3(x, y, Self::adjusted_z(z))
    }
}

impl Camera {
    /// Updates the movement state of the camera based on control input.
    pub fn input_state(&mut self, input: InputState, ignore_move: bool) {
        if !ignore_move && input.main_held {
            if let Some(delta) = input.main_move {
                self.on_moving_main(delta);
            }
        }

        if let Some(delta) = input.scroll {
            self.on_scroll(delta);
        }
    }

    /// Gets the adjusted z-position of the camera.
    fn adjusted_z(z: Double) -> Double {
        if z <= 1.0 {
            return z;
        }

        if z < 1.8 {
            1.0
        } else {
            z - 0.8
        }
    }

    /// Scroll the camera to a new position.
    fn scroll(z: Double, vel: Double) -> Double {
        let z = z + vel * 0.4;

        clamp(z, FAR + 0.1, 3.0)
    }

    /// Updates the camera's position.
    pub fn update_pos(&mut self) {
        let pos = &mut self.pos;

        {
            let vel = &mut self.move_vel;

            if !vel.is_zero() {
                pos.x += vel.x;
                pos.y += vel.y;

                *vel *= 0.9;
            }
        }

        {
            let vel = &mut self.scroll_vel;
            if !vel.is_zero() {
                pos.z = Self::scroll(pos.z, *vel);

                *vel *= 0.6;
            }
        }
    }
}

impl Camera {
    /// Called when the camera is scrolled.
    fn on_scroll(&mut self, delta: DVector2) {
        let y = delta.y;

        if y.abs() > 0.0 {
            let change = -y;

            self.scroll_vel += change * 0.2;
        }
    }

    /// Called when the camera is moving.
    fn on_moving_main(&mut self, delta: DVector2) {
        self.move_vel += delta / 250.0;
    }

    /// Sets the position the camera is centered on.
    pub fn update_pointing_at(&mut self, main_pos: DPoint2, width: Double, height: Double) {
        let p = main_pos_to_hex(width, height, self.get_pos(), main_pos);

        self.pointing_at = p.round().into();
    }

    /// Gets the TileCoord the camera is pointing at.
    pub fn get_tile_coord(&self) -> TileCoord {
        let pos = self.pos;
        let point = point(pos.x, pos.y);

        pixel_to_hex(HEX_LAYOUT, point).round().into()
    }
}

/// Gets the hex position being pointed at..
pub fn main_pos_to_hex(
    width: Double,
    height: Double,
    camera_pos: DPoint3,
    main_pos: DPoint2,
) -> FractionalHex<Double> {
    let p = screen_to_world(width, height, main_pos);
    let p = p + camera_pos.to_vec();

    let p = point(p.x, p.y);

    pixel_to_hex(HEX_LAYOUT, p)
}

/// Converts screen space coordinates into normalized coordinates.
pub fn screen_to_normalized(width: Double, height: Double, c: DPoint2) -> DPoint2 {
    let size = vec2(width, height) / 2.0;

    let c = vec2(c.x, c.y);
    let c = c.zip(size, Sub::sub);
    let c = c.zip(size, Div::div);

    point2(c.x, c.y)
}

/// Converts screen coordinates to world coordinates..
pub fn screen_to_world(width: Double, height: Double, c: DPoint2) -> DPoint3 {
    let c = screen_to_normalized(width, height, c);

    normalized_to_world(width, height, c)
}

/// Converts normalized screen coordinates to world coordinates..
pub fn normalized_to_world(width: Double, height: Double, p: DPoint2) -> DPoint3 {
    let aspect = width / height;

    let matrix = matrix(point3(0.0, 0.0, 1.0), aspect, PI);

    let p = p.to_vec();
    let p = matrix * p.extend(FAR).extend(1.0);
    let p = p.truncate() * p.w;

    let aspect_squared = aspect.powi(2);

    point3(p.x * aspect_squared, p.y, p.z)
}

/// Converts hex coordinates to normalized screen coordinates.
pub fn hex_to_normalized(
    width: Double,
    height: Double,
    camera_pos: DPoint3,
    hex: TileCoord,
) -> DPoint3 {
    let Point { x, y } = hex_to_pixel(HEX_LAYOUT, hex.into());

    let aspect = width / height;

    let matrix = matrix(camera_pos, aspect, PI);

    let p = vec2(x, y);
    let p = matrix * p.extend(FAR).extend(1.0);
    let p = p.truncate() / p.w;

    point3(p.x, p.y, p.z)
}
