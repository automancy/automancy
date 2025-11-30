use core::ops::{Deref, DerefMut};
use std::collections::BTreeMap;

use automancy_data::{
    game::{coord::TileCoord, generic::DataMap},
    id::TileId,
};
use hashbrown::HashMap;

use crate::actor::message::TileMsg;

pub mod game_entity;
pub mod message;
pub mod tile_entity;
pub mod util;

pub type FlatTile = (TileId, DataMap);
pub type FlatTiles = HashMap<TileCoord, FlatTile>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileEntry {
    /// the [`TileId`] of the tile
    pub id: TileId,

    /// the handle to the actor handling this tile
    pub handle: ractor::ActorRef<TileMsg>,
}

#[derive(Debug, Default, Clone)]
pub struct TileMap(BTreeMap<TileCoord, TileEntry>);

impl TileMap {
    pub fn into_inner(self) -> BTreeMap<TileCoord, TileEntry> {
        self.0
    }
}

impl Deref for TileMap {
    type Target = BTreeMap<TileCoord, TileEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TileMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
