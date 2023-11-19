use std::ops::Mul;

use egui::NumExt;

use automancy_defs::cgmath::{point2, point3, vec2, InnerSpace};
use automancy_defs::coord::{TileCoord, TileRange};
use automancy_defs::hexagon_tiles::traits::HexRound;
use automancy_defs::math;
use automancy_defs::math::{DPoint2, DPoint3, DVector2, Double};

use crate::input::InputHandler;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pos: DPoint3,
    move_vel: DVector2,
    scroll_vel: Double,

    pub culling_range: TileRange,
    pub pointing_at: TileCoord,
}

fn fit_z(mut z: Double) -> Double {
    if z > 2.0 {
        if z <= 3.5 {
            z = 2.0
        } else {
            z -= 1.5
        }
    }

    7.5 + z.powi(2) * 1.5
}

fn fit_pos(DPoint3 { x, y, z }: DPoint3) -> DPoint3 {
    point3(x, y, fit_z(z))
}

impl Camera {
    pub fn new((width, height): (Double, Double)) -> Self {
        let pos = point3(0.0, 0.0, 3.0);

        Self {
            pos,
            move_vel: vec2(0.0, 0.0),
            scroll_vel: 0.0,

            culling_range: math::get_culling_range((width, height), fit_pos(pos)),
            pointing_at: TileCoord::new(0, 0),
        }
    }

    /// Returns the position of the camera.
    pub fn get_pos(&self) -> DPoint3 {
        fit_pos(self.pos)
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

        if self.move_vel.magnitude2() > 0.000001 {
            self.pos.x += self.move_vel.x * m;
            self.pos.y += self.move_vel.y * m;

            self.move_vel -= self.move_vel * elapsed.mul(4.0).at_most(0.9);
        }

        if self.scroll_vel.abs() > 0.00005 {
            self.pos.z += self.scroll_vel * m;
            self.pos.z = self.pos.z.clamp(1.0, 4.5);

            self.scroll_vel -= self.scroll_vel * elapsed.mul(15.0).at_most(0.9);
        }

        self.culling_range = math::get_culling_range((width, height), self.get_pos());
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
