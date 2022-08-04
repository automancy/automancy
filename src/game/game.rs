use std::collections::HashMap;

use actix::{Actor, Context, Handler, Message, MessageResponse, Recipient, ResponseFuture};

use cgmath::point2;
use futures::{future::try_join_all, FutureExt};
use serde::{Deserialize, Serialize};

use crate::{
    game::data::{chunk::Chunk, pos::Pos},
    math::data::{Num, Point2, Point3},
};

use super::data::pos::Real;

#[derive(Message)]
#[rtype(result = "Result<Vec<()>, ()>")]
pub struct Tick();

#[derive(MessageResponse)]
pub struct WorldRenderContext {
    pub visible_chunks: Vec<Chunk>,
}

#[derive(Message)]
#[rtype(result = "WorldRenderContext")]
pub struct WorldRenderContextRequest {
    pub pos: Point3,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "Option<()>")]
pub struct GameState {
    pub tick_count: usize,
}

pub struct Game {
    pub loaded_chunks: HashMap<Pos, Chunk>,

    tick_count: usize,

    tick_recipients: Vec<Recipient<GameState>>,
}

impl Actor for Game {
    type Context = Context<Self>;
}

impl Handler<Tick> for Game {
    type Result = ResponseFuture<Result<Vec<()>, ()>>;

    fn handle(&mut self, _msg: Tick, _ctx: &mut Self::Context) -> Self::Result {
        self.tick();

        let mut state = GameState::default();

        state.tick_count = self.tick_count;

        // notify

        // TODO wtf?
        let futures = self
            .tick_recipients
            .clone()
            .into_iter()
            .map(|v| v.send(state.clone()).map(|v| v.unwrap().ok_or(())));

        Box::pin(try_join_all(futures))
    }
}

const SQRT_3: Num = 1.732050807568877293527446341505;

/// assuming size = 1.0
pub fn world_pos_to_screen(pos: Pos) -> Point2 {
    let x = pos.0 as Num;
    let y = pos.1 as Num;
    let odd_row = pos.1 % 2;
    let w = SQRT_3 / 2.0;
    let h = 3.0 / 4.0;

    point2(x * w + (odd_row as f32 * w / 2.0), y * h)
}

/// assuming size = 1.0
pub fn screen_pos_to_world(pos: Point2) -> Pos {
    Pos(
        (pos.x / SQRT_3).round() as Real,
        (pos.y / 2.0).round() as Real,
    )
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LoadChunk(pub Pos);

impl Handler<LoadChunk> for Game {
    type Result = ();

    fn handle(&mut self, msg: LoadChunk, _ctx: &mut Self::Context) -> Self::Result {
        let pos = msg.0;

        if !self.loaded_chunks.contains_key(&pos) {
            self.loaded_chunks.insert(pos, Chunk::load(pos));
        }
    }
}

impl Handler<WorldRenderContextRequest> for Game {
    type Result = WorldRenderContext;

    fn handle(&mut self, msg: WorldRenderContextRequest, _ctx: &mut Self::Context) -> Self::Result {
        let pos = msg.pos;
        let pos = screen_pos_to_world(point2(pos.x, pos.y));
        let (min_pos, max_pos) = (pos - Pos(1, 1), pos + Pos(1, 1));

        let visible_chunks = self
            .loaded_chunks
            .values()
            .filter(|chunk| chunk.pos > min_pos && chunk.pos < max_pos)
            .map(Chunk::clone)
            .collect::<Vec<_>>();

        WorldRenderContext { visible_chunks }
    }
}

impl Game {
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.overflowing_add(1).0;
    }

    pub fn new(tick_recipients: Vec<Recipient<GameState>>) -> Self {
        Self {
            loaded_chunks: HashMap::new(),

            tick_count: 0,

            tick_recipients,
        }
    }
}
