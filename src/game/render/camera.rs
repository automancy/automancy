use std::{
    f32::{
        consts::{FRAC_PI_2, PI},
        EPSILON,
    },
    ops::{Div, Mul, Sub},
};

use cgmath::{point2, point3, vec3, EuclideanSpace, SquareMatrix, Zero};
use tokio::sync::{
    broadcast::{channel, Receiver, Sender},
    watch,
};

use crate::{
    game::{
        data::tile::{TileCoord, TileUnit},
        game::GameState,
        player::input::handler::InputState,
    },
    math::{
        cg::{DisplayCoord, Matrix4, Num, Point3, Vector2, Vector3},
        util::perspective,
    },
};

use super::renderer::RendererState;

pub const MAX_CAMERA_Z: Num = 4.0;

#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    pub pos: Point3,
    pub matrix: Matrix4,
    pub pointing_at: TileCoord,
}

pub struct Camera {
    pos: Point3,
    matrix: Matrix4,
    pointing_at: TileCoord,

    send_camera_state: Sender<CameraState>,
    recv_input_state: watch::Receiver<Option<InputState>>,
    recv_game_state: Receiver<GameState>,
    recv_renderer_state: Receiver<RendererState>,

    holding_main: bool,
    move_vel: Vector2,
    scroll_vel: Num,
}

impl Camera {
    pub fn new(
        recv_input_state: watch::Receiver<Option<InputState>>,
        recv_game_state: Receiver<GameState>,
        recv_renderer_state: Receiver<RendererState>,
    ) -> (Self, Receiver<CameraState>) {
        let (send_camera_state, recv_camera_state) = channel(2);

        let it = Self {
            pos: point3(0.0, 0.0, MAX_CAMERA_Z),
            matrix: Matrix4::identity(),
            pointing_at: TileCoord::new(0, 0),

            send_camera_state,
            recv_input_state,
            recv_game_state,
            recv_renderer_state,

            holding_main: false,
            move_vel: Vector2::zero(),
            scroll_vel: 0.0,
        };

        (it, recv_camera_state)
    }

    pub fn send(&self) {
        self.send_camera_state
            .send(CameraState {
                pos: self.pos,
                matrix: self.matrix,
                pointing_at: self.pointing_at,
            })
            .unwrap();
    }

    pub async fn recv(&mut self) {
        let input_state = *self.recv_input_state.borrow_and_update();

        if let Some(state) = input_state {
            self.recv_input_state(state);
        }

        if let Ok(state) = self.recv_game_state.recv().await {
            self.recv_game_state(state);
        }

        if let Ok(state) = self.recv_renderer_state.recv().await {
            self.recv_renderer_state(state);
        }
    }
}

impl Camera {
    fn recv_input_state(&mut self, state: InputState) {
        if let Some(_) = state.main_hold {
            self.on_holding_main();
        } else {
            self.on_not_holding_main();
        }

        if let Some(delta) = state.main_move {
            self.on_moving_main(delta);
        }

        if let Some(delta) = state.scroll {
            self.on_scroll(delta);
        }
    }

    fn recv_game_state(&mut self, _state: GameState) {
        let pos = &mut self.pos;

        {
            let vel = &mut self.move_vel;
            if !vel.is_zero() {
                pos.x += vel.x;
                pos.y += vel.y;

                *vel -= *vel * 0.05;
            }
        }

        {
            let vel = &mut self.scroll_vel;
            if !vel.is_zero() {
                pos.z = scroll(pos.z, *vel);

                *vel -= *vel * 0.2;
            }
        }
    }

    pub fn point3_to_tile_coord(p: Point3) -> TileCoord {
        let pos = DisplayCoord::from_point2(point2(p.x, p.y)).to_cube_as_pointy_top();

        TileCoord::new(pos.q() as TileUnit, pos.r() as TileUnit)
    }

    fn recv_renderer_state(&mut self, state: RendererState) {
        self.update_matrix(state.aspect);

        let matrix = self.matrix.invert().unwrap();

        let size = state.window_size.to_vec() / 2.0;
        let c = state.cursor_pos.to_vec();
        let c = c.zip(size, Sub::sub);
        let c = c.zip(size, Div::div);

        let v = c.extend(1.0);
        let v = matrix * v.extend(1.0);
        let v = v.truncate() / v.w;

        let p = point3(v.x, v.y, v.z);

        self.pointing_at = Self::point3_to_tile_coord(p);
    }
}

fn scroll(z: Num, vel: Num) -> Num {
    let z = z + vel;

    if z < EPSILON {
        return 0.0;
    }

    if z >= MAX_CAMERA_Z {
        return MAX_CAMERA_Z;
    }

    return z;
}

impl Camera {
    fn update_matrix(&mut self, aspect: Num) {
        let view = Self::view(self.pos);
        let projection = Self::projection(aspect);

        let matrix = projection * view;

        self.matrix = matrix;
    }

    fn eye(pos: Point3) -> Vector3 {
        let z = 1.0 - pos.z.div(MAX_CAMERA_Z);
        let r = -z.mul(FRAC_PI_2).sin();
        let o = r.mul(PI / 2.25).cos();

        vec3(0.0, r, o)
    }

    fn actual_pos(pos: Point3, eye: Vector3) -> Point3 {
        point3(pos.x, -pos.y, pos.z + eye.z)
    }

    fn view(pos: Point3) -> Matrix4 {
        let eye = Self::eye(pos);
        let actual_pos = Self::actual_pos(pos, eye);

        let view = Matrix4::look_to_rh(actual_pos, eye, -Vector3::unit_y());

        view
    }

    fn projection(aspect: Num) -> Matrix4 {
        perspective(FRAC_PI_2, aspect, 0.1, 100.0)
    }
}

impl Camera {
    fn on_scroll(&mut self, delta: Vector2) {
        let y = delta.y;

        if y.abs() > 0.0 {
            let change = -y;

            self.scroll_vel += change / 7.5;
        }
    }

    fn on_holding_main(&mut self) {
        if !self.holding_main {
            self.holding_main = true;
        }
    }

    fn on_not_holding_main(&mut self) {
        if self.holding_main {
            self.holding_main = false;
        }
    }

    fn on_moving_main(&mut self, delta: Vector2) {
        if self.holding_main {
            self.move_vel += delta / 1500.0;
        }
    }
}
