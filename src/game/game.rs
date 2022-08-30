use std::{collections::HashMap, sync::Arc};

use actix::{Actor, Context, Handler, Message, MessageResponse, Recipient, ResponseFuture};

use futures::{future::try_join_all, FutureExt};
use serde::{Deserialize, Serialize};

use crate::game::data::{chunk::RawChunk, pos::Pos};

use super::data::{chunk::Chunk, grid::to_xyz, id::Id};

#[derive(Debug, Default, Clone, Message)]
#[rtype(result = "Option<()>")]
pub struct GameState {
    pub tick_count: usize,
}

pub struct Game {
    pub loaded_chunks: HashMap<Pos, Arc<Chunk>>,

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

#[derive(Message)]
#[rtype(result = "()")]
pub struct LoadChunk(pub Pos);

impl Handler<LoadChunk> for Game {
    type Result = ();

    fn handle(&mut self, msg: LoadChunk, _ctx: &mut Self::Context) -> Self::Result {
        self.load_chunk(msg.0)
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LoadChunkRange(pub Pos, pub Pos);

impl Handler<LoadChunkRange> for Game {
    type Result = ();

    fn handle(&mut self, msg: LoadChunkRange, _ctx: &mut Self::Context) -> Self::Result {
        self.load_chunk_range(msg.0, msg.1);
    }
}

#[derive(Message)]
#[rtype(result = "Result<Vec<()>, ()>")]
pub struct Tick();

#[derive(MessageResponse)]
pub struct WorldRenderContext {
    pub visible_chunks: Vec<Arc<Chunk>>,
}

#[derive(Message)]
#[rtype(result = "WorldRenderContext")]
pub struct WorldRenderContextRequest {
    pub pos: Pos,
    pub range: isize,
}

impl Handler<WorldRenderContextRequest> for Game {
    type Result = WorldRenderContext;

    fn handle(&mut self, msg: WorldRenderContextRequest, _ctx: &mut Self::Context) -> Self::Result {
        let pos = msg.pos;
        let range = msg.range;

        let (min_pos, max_pos) = (pos - Pos(range, range), pos + Pos(range, range));

        let visible_chunks = self
            .loaded_chunks
            .values()
            .filter(|chunk| chunk.pos >= min_pos && chunk.pos <= max_pos)
            .map(Arc::clone)
            .collect::<Vec<_>>();

        WorldRenderContext { visible_chunks }
    }
}

impl Game {
    pub fn load_chunk_range(&mut self, min: Pos, max: Pos) {
        let (min, max) = (min.min(max), min.max(max));

        for i in min.0..=max.0 {
            for j in min.1..=max.1 {
                self.load_chunk(Pos(i, j));
            }
        }
    }

    fn gen_chunk(&mut self, chunk: &mut Chunk) {
        chunk.tiles.iter_mut().enumerate().for_each(|(idx, tile)| {
            if to_xyz(idx as isize).2 == 0 {
                tile.id = Id::automancy("tile".to_string());
            }
        });
    }

    pub fn load_chunk(&mut self, pos: Pos) {
        if !self.loaded_chunks.contains_key(&pos) {
            let raw_chunk = RawChunk::load(pos);
            let mut chunk = Chunk::from_raw(raw_chunk);

            self.gen_chunk(&mut chunk);

            let chunk = Arc::new(chunk);

            self.loaded_chunks.insert(pos, chunk);
        }
    }

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
