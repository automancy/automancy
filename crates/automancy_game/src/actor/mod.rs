use core::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use automancy_data::{
    game::{coord::TileCoord, generic::DataMap},
    id::TileId,
};
use hashbrown::HashMap;

use crate::actor::message::TileMsg;

pub mod game;
pub mod map;
pub mod message;
pub mod tile;
pub mod util;

/// Game ticks per second
pub const TPS: u64 = 60;
pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
pub const MAX_ALLOWED_TICK_INTERVAL: Duration = TICK_INTERVAL.saturating_mul(5);

pub const TRANSACTION_ANIMATION_SPEED: Duration = Duration::from_nanos(800_000_000);
pub const TRANSACTION_MIN_INTERVAL: Duration = Duration::from_nanos(250_000_000);
pub const TAKE_ITEM_ANIMATION_SPEED: Duration = Duration::from_nanos(300_000_000);

const UNDO_CACHE_SIZE: usize = 256;

pub type TickUnit = u16;

pub type FlatTiles = HashMap<TileCoord, (TileId, DataMap)>;

#[derive(Debug, Clone)]
pub struct TileEntry {
    /// the [`TileId`] of the tile
    pub id: TileId,

    /// the handle to the actor handling this tile
    pub handle: ractor::ActorRef<TileMsg>,
}

#[derive(Debug, Default, Clone)]
pub struct TileMap(HashMap<TileCoord, TileEntry>);

impl TileMap {
    pub fn into_inner(self) -> HashMap<TileCoord, TileEntry> {
        self.0
    }
}

impl Deref for TileMap {
    type Target = HashMap<TileCoord, TileEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TileMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
