use std::f64::consts::PI;
use std::ops::{Div, Sub};
use cgmath::{EuclideanSpace, point2, point3, vec2, Zero};
use cgmath::num_traits::clamp;
use hexagon_tiles::layout::pixel_to_hex;
use hexagon_tiles::point::{point, Point};
use hexagon_tiles::traits::HexRound;
use crate::data::tile::TileCoord;

use crate::render::data::RENDER_LAYOUT;
use crate::input::handler::InputState;
use crate::math::cg::{Double, DPoint2, DPoint3, DVector2, matrix};

pub const FAR: Double = 0.0;

#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    pub pos: DPoint3,
    pub holding_main: bool,
    pub move_vel: DVector2,
    pub scroll_vel: Double,
    pub main_pos: DPoint2,
    pub pointing_at: TileCoord,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pos: point3(0.0, 0.0, 1.0),
            holding_main: false,
            move_vel: vec2(0.0, 0.0),
            scroll_vel: 0.0,
            main_pos: point2(0.0, 0.0),
            pointing_at: TileCoord::new(0, 0),
        }
    }
}

impl CameraState {
    pub fn is_at_max_height(&self) -> bool {
        self.pos.z > 0.98
    }
}

pub struct Camera {
    camera_state: CameraState,

    pub window_size: (Double, Double)
}

impl Camera {
    pub fn new(window_size: (Double, Double)) -> Self {
        Self {
            camera_state: Default::default(),

            window_size,
        }
    }
}

impl Camera {
    pub fn input_state(&mut self, input_state: InputState) {
        self.camera_state.holding_main = input_state.main_pressed;

        if self.camera_state.main_pos != input_state.main_pos {
            self.camera_state.main_pos = input_state.main_pos;
        }

        if self.camera_state.holding_main {
            if let Some(delta) = input_state.main_move {
                self.on_moving_main(delta);
            }
        }

        if let Some(delta) = input_state.scroll {
            self.on_scroll(delta);
        }
    }

    fn scroll(z: Double, vel: Double) -> Double {
        let z = z + vel * 0.4;

        clamp(z, FAR, 1.0)
    }

    pub fn camera_state(&self) -> CameraState {
        self.camera_state
    }

    pub fn update_pos(&mut self) {
        let pos = &mut self.camera_state.pos;

        {
            let vel = &mut self.camera_state.move_vel;

            if !vel.is_zero() {
                pos.x += vel.x;
                pos.y += vel.y;

                *vel *= 0.9;
            }
        }

        {
            let vel = &mut self.camera_state.scroll_vel;
            if !vel.is_zero() {
                pos.z = Self::scroll(pos.z, *vel);

                *vel *= 0.7;
            }
        }
    }
}

impl Camera {
    fn on_scroll(&mut self, delta: DVector2) {
        let y = delta.y;

        if y.abs() > 0.0 {
            let change = -y;

            self.camera_state.scroll_vel += change * 0.2;
        }
    }

    fn on_moving_main(&mut self, delta: DVector2) {
        self.camera_state.move_vel += delta / 250.0;
    }

    pub fn cursor_to_pos(&self, pos: DPoint3) -> DPoint2 {
        let (width, height) = self.window_size;
        let size = vec2(width, height) / 2.0;
        let aspect = width / height;

        let c = self.camera_state.main_pos;
        let c = vec2(c.x, c.y);
        let c = c.zip(size, Sub::sub);
        let c = c.zip(size, Div::div);
        let c = point3(c.x, c.y, FAR);

        let matrix = matrix(pos, aspect, PI);

        let v = c.to_vec();
        let v = matrix * v.extend(1.0);
        let v = v.truncate().truncate() * v.w;

        let aspect_squared = aspect.powi(2);

        point2(v.x * aspect_squared, v.y)
    }

    pub fn update_pointing_at(&mut self) {
        let camera_pos = self.camera_state.pos;
        let pos = point(camera_pos.x, camera_pos.y);
        let pos = pixel_to_hex(RENDER_LAYOUT, pos);

        let p = self.cursor_to_pos(point3(0.0, 0.0, self.camera_state.pos.z));
        let p = point(p.x, p.y);
        let p = pixel_to_hex(RENDER_LAYOUT, p);
        let p = p + pos;

        self.camera_state.pointing_at = TileCoord(p.round());
    }
}
