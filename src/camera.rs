use std::time::Duration;

use num::{clamp, Zero};

use automancy_defs::cg::{DPoint2, DPoint3, DVector2, Double};
use automancy_defs::cgmath::{point2, point3, vec2};
use automancy_defs::coord::{TileCoord, TileHex, TileUnit};
use automancy_defs::hexagon_tiles::fractional::FractionalHex;
use automancy_defs::hexagon_tiles::layout::pixel_to_hex;
use automancy_defs::hexagon_tiles::point::point;
use automancy_defs::hexagon_tiles::traits::HexRound;
use automancy_defs::rendering::HEX_GRID_LAYOUT;

use crate::input::InputHandler;
use crate::util::render::{main_pos_to_hex, normalized_to_world};

pub const FAR: Double = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pos: DPoint3,
    move_vel: DVector2,
    scroll_vel: Double,

    pub culling_range: (TileUnit, TileUnit),
    pub pointing_at: TileCoord,
}

/// Gets the culling range from the camera's position
pub fn get_culling_range((width, height): (Double, Double), z: Double) -> (TileUnit, TileUnit) {
    let a = normalized_to_world((width, height), point2(-1.0, -1.0), z);
    let b = normalized_to_world((width, height), point2(1.0, 1.0), z);

    let a = pixel_to_hex(HEX_GRID_LAYOUT, point(a.x, a.y));
    let b = pixel_to_hex(HEX_GRID_LAYOUT, point(b.x, b.y));

    let a = a + FractionalHex::new(2.0, -2.0);
    let b = b + FractionalHex::new(-2.0, 2.0);

    let o = b - a;
    let o: TileHex = o.round();

    (o.q().abs(), o.r().abs())
}

impl Camera {
    pub fn new((width, height): (Double, Double)) -> Self {
        Self {
            pos: point3(0.0, 0.0, 1.0),
            move_vel: vec2(0.0, 0.0),
            scroll_vel: 0.0,

            culling_range: get_culling_range((width, height), Self::get_z(1.0)),
            pointing_at: TileCoord::new(0, 0),
        }
    }

    pub fn get_z(z: Double) -> Double {
        (z + 3.0) * 3.0
    }

    /// Returns the position of the camera.
    pub fn get_pos(&self) -> DPoint3 {
        let DPoint3 { x, y, z } = self.pos;

        point3(x, y, Self::get_z(z))
    }
}

impl Camera {
    /// Updates the movement state of the camera based on control input.
    pub fn handle_input(&mut self, input: &InputHandler, ignore_move: bool) {
        if !ignore_move && input.main_held {
            if let Some(delta) = input.main_move {
                self.on_moving_main(delta);
            }
        }

        if let Some(delta) = input.scroll {
            self.on_scroll(delta);
        }
    }

    /// Scroll the camera to a new position.
    fn scroll(z: Double, vel: Double, ratio: Double) -> Double {
        let z = z + vel * ratio * 0.6;

        clamp(z, 1.0, 5.0)
    }

    /// Updates the camera's position.
    pub fn update_pos(&mut self, elapsed: Duration, (width, height): (Double, Double)) {
        let ratio = elapsed.as_secs_f64() * 80.0;

        {
            if !self.move_vel.is_zero() {
                self.pos.x += self.move_vel.x * ratio;
                self.pos.y += self.move_vel.y * ratio;

                self.move_vel *= 0.9;
            }
        }

        {
            if !self.scroll_vel.is_zero() {
                self.pos.z = Self::scroll(self.pos.z, self.scroll_vel, ratio);

                self.scroll_vel *= 0.6;
            }
        }

        self.culling_range = get_culling_range((width, height), self.get_pos().z);
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
    pub fn update_pointing_at(&mut self, main_pos: DPoint2, (width, height): (Double, Double)) {
        let p = main_pos_to_hex((width, height), self.get_pos(), main_pos);

        self.pointing_at = p.round().into();
    }

    /// Gets the TileCoord the camera is pointing at.
    pub fn get_tile_coord(&self) -> TileCoord {
        let pos = self.pos;
        let point = point(pos.x, pos.y);

        pixel_to_hex(HEX_GRID_LAYOUT, point).round().into()
    }
}
