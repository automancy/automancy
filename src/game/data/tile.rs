use hexagon_tiles::hexagon::Hex;
use crate::{
    util::id::{id_static, Id},
};

use super::data::Data;

use serde::{Deserialize, Serialize};

pub const NONE: Id = id_static("automancy", "none");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub id: Id,
    pub data: Data,
}

pub type TileUnit = i32;

pub type TileCoord = Hex;