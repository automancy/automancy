use std::ops::Mul;

use egui::NumExt;
use num::Zero;

use automancy_defs::cgmath::{point2, point3, vec2};
use automancy_defs::coord::{TileCoord, TileUnit};
use automancy_defs::hexagon_tiles::traits::HexRound;
use automancy_defs::math;
use automancy_defs::math::{DPoint2, DPoint3, DVector2, Double};

use crate::input::InputHandler;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pos: DPoint3,
    move_vel: DVector2,
    scroll_vel: Double,

    pub culling_range: (TileUnit, TileUnit),
    pub pointing_at: TileCoord,
}

impl Camera {
    pub fn new((width, height): (Double, Double)) -> Self {
        Self {
            pos: point3(0.0, 0.0, 1.0),
            move_vel: vec2(0.0, 0.0),
            scroll_vel: 0.0,

            culling_range: math::get_culling_range((width, height), Self::get_z(1.0)),
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
    /// Sets the position the camera is centered on.
    pub fn update_pointing_at(&mut self, main_pos: DPoint2, (width, height): (Double, Double)) {
        let p = math::main_pos_to_hex((width, height), self.get_pos(), main_pos);

        self.pointing_at = p.round().into();
    }

    /// Gets the TileCoord the camera is pointing at.
    pub fn get_tile_coord(&self) -> TileCoord {
        math::pixel_to_hex(point2(self.pos.x, self.pos.y))
            .round()
            .into()
    }

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

    /// Updates the camera's position.
    pub fn update_pos(&mut self, (width, height): (Double, Double), elapsed: Double) {
        let m = elapsed * 100.0;

        if !self.move_vel.is_zero() {
            self.pos.x += self.move_vel.x * m;
            self.pos.y += self.move_vel.y * m;

            self.move_vel -= self.move_vel * elapsed.mul(4.0).at_most(0.9);
        }

        if !self.scroll_vel.is_zero() {
            self.pos.z += self.scroll_vel * m;

            self.pos.z = self.pos.z.clamp(1.0, 5.0);

            self.scroll_vel -= self.scroll_vel * elapsed.mul(15.0).at_most(0.9);
        }

        self.culling_range = math::get_culling_range((width, height), self.get_pos().z);
    }

    /// Called when the camera is scrolled.
    fn on_scroll(&mut self, delta: DVector2) {
        const MAX_SCROLL_VEL: Double = 0.2;

        let y = delta.y;

        if y != 0.0 {
            let change = -y;

            self.scroll_vel += change / 20.0;
            self.scroll_vel = self.scroll_vel.clamp(-MAX_SCROLL_VEL, MAX_SCROLL_VEL);
        }
    }

    /// Called when the camera is moving.
    fn on_moving_main(&mut self, delta: DVector2) {
        const MAX_MOVE_VEL: Double = 2.0;

        self.move_vel += delta / 600.0;
        self.move_vel = self.move_vel.map(|v| v.clamp(-MAX_MOVE_VEL, MAX_MOVE_VEL));
    }
}
