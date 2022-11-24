use crate::math::hex::cube::CubeCoord;

use super::{data::Data, id::Id};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tile {
    pub id: Id,
    pub data: Data,
}

pub type TileUnit = isize;

pub type TileCoord = CubeCoord<TileUnit>;
