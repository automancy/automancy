use std::{
    f32::{
        consts::{FRAC_PI_2, PI},
        EPSILON,
    },
    ops::{Div, Mul, Neg, Sub},
    sync::Arc,
};

use actix::{Actor, Context, Handler, Message, MessageResponse};
use cgmath::{point3, vec2, vec3, vec4, EuclideanSpace, InnerSpace, SquareMatrix, Transform, Zero};
use collision::Discrete;

use crate::{
    game::{
        data::{
            chunk::{Chunk, UsableTile},
            grid::{
                real_pos_to_world, world_pos_to_real, world_pos_to_real_point, NUM_X, NUM_Y,
                WORLD_SPACING, WORLD_X_SPACING, WORLD_Y_SPACING,
            },
            pos::{Pos, Real},
        },
        game::GameState,
        player::input_handler::InputState,
    },
    math::{
        data::{Aabb3, Matrix4, Num, Point2, Point3, Ray3, Vector2, Vector3, Vector4},
        util::perspective,
    },
    registry::init::InitData,
};

pub const MAX_CAMERA_Z: Num = 4.0;

#[derive(Message)]
#[rtype(result = "View")]
pub struct ViewRequest {
    pub aspect: Num,
}

#[derive(Debug, Clone, MessageResponse)]
pub struct View(pub Matrix4);

#[derive(Message)]
#[rtype(result = "CameraPos")]
pub struct CameraPosRequest;

#[derive(Debug, Clone, MessageResponse)]
pub struct CameraPos(pub Point3);

#[derive(Message)]
#[rtype(result = "CameraRay")]
pub struct CameraRayRequest {
    pub aspect: Num,
    pub size: Point2,
    pub pos: Point2,
    pub visible_chunks: Vec<Arc<Chunk>>,
    pub init_data: Arc<InitData>,
}

#[derive(Debug, Clone, MessageResponse)]
pub struct CameraRay {
    pub result: Vec<(Arc<Chunk>, usize, UsableTile)>,
    pub p: Vector2,
}

pub struct Camera {
    pos: Point3,

    holding_main: bool,
    move_vel: Vector2,
    scroll_vel: Num,
}

impl Actor for Camera {
    type Context = Context<Self>;
}

impl Handler<ViewRequest> for Camera {
    type Result = View;

    fn handle(&mut self, msg: ViewRequest, _ctx: &mut Self::Context) -> Self::Result {
        View(Self::matrix(&self.pos, msg.aspect).0)
    }
}

impl Handler<CameraPosRequest> for Camera {
    type Result = CameraPos;

    fn handle(&mut self, _msg: CameraPosRequest, _ctx: &mut Self::Context) -> Self::Result {
        CameraPos(self.pos)
    }
}

impl Handler<CameraRayRequest> for Camera {
    type Result = CameraRay;

    fn handle(&mut self, msg: CameraRayRequest, _ctx: &mut Self::Context) -> Self::Result {
        let aspect = msg.aspect;

        let (p, c) = {
            let pos = self.pos;

            let matrix = Self::matrix(&pos, aspect).0;

            let size = msg.size.to_vec() / 2.0;
            let c = msg.pos.to_vec();
            let c = c.zip(size, Sub::sub);
            let c = c.zip(size, Div::div);

            let p = c.extend((MAX_CAMERA_Z - pos.z) / MAX_CAMERA_Z);
            let p = matrix * p.extend(1.0);
            let mut p = p.truncate().truncate() * p.w;
            p.x *= aspect * aspect;

            let x = matrix * vec4(0.0, 0.0, -1.0, aspect);
            let x = x.x * x.w;

            let y = matrix * vec4(0.0, 0.0, 1.0, 1.0);
            let y = y.y * y.w;

            (p - vec2(x, y), c)
        };

        let (eye, z) = {
            let pos = self.pos;
            let (matrix, _, actual_pos) = Self::matrix(&pos, aspect);

            //let o = vec4(0.0, 0.0, 0.0, 1.0);
            //let eye = matrix * o;
            ////let mut eye = eye.truncate() * eye.w;
            //let mut eye = eye.truncate();
            let eye = Self::eye(&pos);

            (eye, actual_pos.z)
        };

        let ray = Ray3::new(point3(p.x, p.y, z), -eye);

        let visible_chunks = msg.visible_chunks;
        let init_data = msg.init_data;

        let result = visible_chunks
            .iter()
            .flat_map(|chunk| {
                chunk
                    .tiles
                    .iter()
                    .enumerate()
                    .find(|(index, v)| {
                        if let Some(bound) =
                            &init_data.all_bounding_boxes[init_data.resources_map[&v.id]]
                        {
                            let pos = chunk.real_pos_to_world(*index).to_vec();

                            let a = bound.min + pos;
                            let b = bound.max + pos;

                            let bound = Aabb3::new(a, b);

                            ray.intersects(&bound)
                        } else {
                            false
                        }
                    })
                    .map(|(idx, tile)| (chunk.clone(), idx, tile.clone()))
            })
            .collect();

        CameraRay { result, p }
    }
}

impl Handler<InputState> for Camera {
    type Result = Option<()>;

    fn handle(&mut self, state: InputState, _ctx: &mut Self::Context) -> Self::Result {
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

        Some(())
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

impl Handler<GameState> for Camera {
    type Result = Option<()>;

    fn handle(&mut self, _msg: GameState, _ctx: &mut Self::Context) -> Self::Result {
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

        Some(())
    }
}

impl Camera {
    pub fn new() -> Self {
        Self {
            pos: point3(0.0, 0.0, MAX_CAMERA_Z),

            holding_main: false,
            move_vel: Vector2::zero(),
            scroll_vel: 0.0,
        }
    }

    fn eye(pos: &Point3) -> Vector3 {
        let z = 1.0 - pos.z.div(MAX_CAMERA_Z);
        let r = -z.mul(FRAC_PI_2).sin();
        let o = r.mul(PI / 2.25).cos();

        vec3(0.0, r, o)
    }

    fn actual_pos(pos: &Point3, eye: &Vector3) -> Point3 {
        point3(pos.x, -pos.y, pos.z + eye.z)
    }

    fn view(pos: &Point3) -> (Matrix4, Point3) {
        let eye = Self::eye(pos);
        let actual_pos = Self::actual_pos(pos, &eye);
        let view = Matrix4::look_to_rh(actual_pos, eye, -Vector3::unit_y());

        (view, actual_pos)
    }

    fn projection(aspect: Num) -> Matrix4 {
        perspective(FRAC_PI_2, aspect, 0.1, 100.0)
    }

    fn matrix(pos: &Point3, aspect: Num) -> (Matrix4, Matrix4, Point3) {
        let (view, actual_pos) = Self::view(pos);
        let projection = Self::projection(aspect);

        (projection * view, projection, actual_pos)
    }

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

    pub fn camera_pos_to_real(pos: Point3) -> Pos {
        world_pos_to_real(vec2(pos.x / NUM_X, -pos.y / NUM_Y))
    }
}
