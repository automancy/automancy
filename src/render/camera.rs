use std::f64::consts::PI;
use std::ops::{Div, Sub};

use cgmath::num_traits::clamp;
use cgmath::{point2, point3, vec2, EuclideanSpace, Zero};
use hexagon_tiles::fractional::FractionalHex;
use hexagon_tiles::layout::{hex_to_pixel, pixel_to_hex};
use hexagon_tiles::point::{point, Point};
use hexagon_tiles::traits::HexRound;

use crate::game::input::InputState;
use crate::game::tile::coord::TileCoord;
use crate::render::data::HEX_LAYOUT;
use crate::util::cg::{matrix, DPoint2, DPoint3, DVector2, Double};

pub const FAR: Double = 0.0;

#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    pub pos: DPoint3,
    pub move_vel: DVector2,
    pub scroll_vel: Double,
    pub pointing_at: TileCoord,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pos: point3(0.0, 0.0, 1.0),
            move_vel: vec2(0.0, 0.0),
            scroll_vel: 0.0,
            pointing_at: TileCoord::new(0, 0),
        }
    }
}

impl CameraState {
    pub fn is_at_max_height(&self) -> bool {
        self.pos.z > 0.9985
    }
}

pub struct Camera {
    camera_state: CameraState,

    pub window_size: (Double, Double),
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

    fn scroll(z: Double, vel: Double) -> Double {
        let z = z + vel * 0.4;

        clamp(z, FAR + 0.2, 1.0)
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

                *vel *= 0.6;
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

    pub fn update_pointing_at(&mut self, main_pos: DPoint2) {
        let (width, height) = self.window_size;

        let p = main_pos_to_hex(width, height, self.camera_state.pos, main_pos);

        self.camera_state.pointing_at = p.round().into();
    }

    pub fn get_tile_coord(&self) -> TileCoord {
        let pos = self.camera_state.pos;
        let point = point(pos.x, pos.y);

        pixel_to_hex(HEX_LAYOUT, point).round().into()
    }
}

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

pub fn screen_to_normalized(width: Double, height: Double, c: DPoint2) -> DPoint2 {
    let size = vec2(width, height) / 2.0;

    let c = vec2(c.x, c.y);
    let c = c.zip(size, Sub::sub);
    let c = c.zip(size, Div::div);

    point2(c.x, c.y)
}

pub fn screen_to_world(width: Double, height: Double, c: DPoint2) -> DPoint3 {
    let c = screen_to_normalized(width, height, c);

    normalized_to_world(width, height, c)
}

pub fn normalized_to_world(width: Double, height: Double, p: DPoint2) -> DPoint3 {
    let aspect = width / height;

    let matrix = matrix(point3(0.0, 0.0, 1.0), aspect, PI);

    let p = p.to_vec();
    let p = matrix * p.extend(FAR).extend(1.0);
    let p = p.truncate() * p.w;

    let aspect_squared = aspect.powi(2);

    point3(p.x * aspect_squared, p.y, p.z)
}

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
