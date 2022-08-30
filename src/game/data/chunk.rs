use std::{fs::File, path::Path};

use cgmath::point3;
use zstd::stream::{copy_decode, copy_encode};

use crate::{
    game::{data::raw::CanBeRaw, render::data::InstanceData},
    math::data::{Num, Point3},
    registry::init::InitData,
};

use super::{
    data::{Data, RawData},
    grid::{real_pos_to_world, to_xyz, Grid, GRID_SIZE, ISIZE_X, ISIZE_Y},
    id::{Id, RawId},
    pos::Pos,
    raw::Raw,
    tile::{RawTile, Tile},
};

pub type RawTiles = Grid<Tile, RawTile>;
pub type RawIdPool = Grid<Id, RawId>;
pub type RawDataPool = Grid<Data, RawData>;

#[derive(Debug, Clone)]
pub struct RawChunk {
    pub pos: Pos,
    pub tiles: RawTiles,
    pub id_pool: RawIdPool,
    pub data_pool: RawDataPool,
}

impl RawChunk {
    pub fn tiles_path(pos: &Pos) -> String {
        format!("map/{}.tiles", pos)
    }

    pub fn data_pool_path(pos: &Pos) -> String {
        format!("map/{}.datapool", pos)
    }

    pub fn id_pool_path(pos: &Pos) -> String {
        format!("map/{}.idpool", pos)
    }

    fn load_part<T: CanBeRaw<R> + Clone + Default, R: Raw>(path: &str) -> Grid<T, R> {
        let file = File::open(path);
        let file = if let Ok(it) = file {
            it
        } else {
            return Grid::default();
        };

        let mut data = Vec::new();
        copy_decode(file, &mut data).unwrap();

        let data = T::map_to_reals(&data);
        Grid::new(data)
    }

    fn load_tiles(pos: &Pos) -> RawTiles {
        Self::load_part(Self::tiles_path(pos).as_str())
    }

    fn load_data_pool(pos: &Pos) -> RawDataPool {
        Self::load_part(Self::data_pool_path(pos).as_str())
    }

    fn load_id_pool(pos: &Pos) -> RawIdPool {
        Self::load_part(Self::id_pool_path(pos).as_str())
    }

    fn unload_part<T: CanBeRaw<R> + Clone + Default, R: Raw>(path: &str, data: Grid<T, R>) {
        let file = File::create(path).unwrap();

        let data: Vec<u8> = data.into();
        copy_encode(data.as_slice(), file, 0).unwrap();
    }

    fn unload_tiles(pos: &Pos, tiles: RawTiles) {
        Self::unload_part(Self::tiles_path(pos).as_str(), tiles)
    }

    fn unload_data_pool(pos: &Pos, data_pool: RawDataPool) {
        Self::unload_part(Self::data_pool_path(pos).as_str(), data_pool)
    }

    fn unload_id_pool(pos: &Pos, id_pool: RawIdPool) {
        Self::unload_part(Self::id_pool_path(pos).as_str(), id_pool)
    }

    pub fn unload(self) {
        std::fs::create_dir_all(Path::new("map")).unwrap();

        Self::unload_tiles(&self.pos, self.tiles);
        Self::unload_data_pool(&self.pos, self.data_pool);
        Self::unload_id_pool(&self.pos, self.id_pool);
    }

    pub fn load(chunk_pos: Pos) -> Self {
        let tiles = Self::load_tiles(&chunk_pos);
        let id_pool = Self::load_id_pool(&chunk_pos);
        let data_pool = Self::load_data_pool(&chunk_pos);

        Self {
            pos: chunk_pos,
            tiles,
            id_pool,
            data_pool,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UsableTile {
    pub id: Id,
    pub data: Data,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub pos: Pos,

    pub tiles: Vec<UsableTile>,
}

impl Chunk {
    pub fn new_empty(pos: Pos) -> Self {
        let mut tiles = Vec::with_capacity(GRID_SIZE);
        tiles.resize(GRID_SIZE, UsableTile::default());

        Self { pos, tiles }
    }

    pub fn new(pos: Pos, mut tiles: Vec<UsableTile>) -> Self {
        tiles.resize(GRID_SIZE, UsableTile::default());

        Self { pos, tiles }
    }

    // TODO doesn't help compression...
    pub fn to_raw(self) -> RawChunk {
        let tiles = Grid::new(
            self.tiles
                .iter()
                .enumerate()
                .map(|v| Tile {
                    id_handle: v.0 as u8,
                    data_handle: v.0 as u8,
                })
                .collect(),
        );

        let id_pool = Grid::new(self.tiles.iter().map(|v| v.id.clone()).collect());
        let data_pool = Grid::new(self.tiles.iter().map(|v| v.data.clone()).collect());

        RawChunk {
            pos: self.pos,
            tiles,
            id_pool,
            data_pool,
        }
    }

    pub fn from_raw(chunk: RawChunk) -> Self {
        Self::new(
            chunk.pos,
            chunk
                .tiles
                .data
                .into_iter()
                .map(|tile| UsableTile {
                    id: chunk.id_pool.data[tile.id_handle as usize].to_owned(),
                    data: chunk.data_pool.data[tile.data_handle as usize].to_owned(),
                })
                .collect(),
        )
    }

    pub fn real_pos(&self, index: usize) -> (Pos, isize) {
        let pos = self.pos;
        let (gx, gy) = (pos.0 * ISIZE_X, pos.1 * ISIZE_Y);

        let (x, y, z) = to_xyz(index as isize);

        (Pos(x + gx, y + gy), z)
    }

    pub fn real_pos_to_world(&self, index: usize) -> Point3 {
        let (pos, z) = self.real_pos(index);

        let z = z as Num;

        let pos = real_pos_to_world(pos);
        point3(pos.x, pos.y, z)
    }

    pub fn tile_to_instance(
        &self,
        index: usize,
        tile: &UsableTile,
        init_data: &InitData,
    ) -> InstanceData {
        let pos = self.real_pos_to_world(index);

        InstanceData::new()
            .position_offset([pos.x, pos.y, pos.z])
            .faces_index(init_data.resources_map[&tile.id])
    }

    pub fn to_instances(&self, init_data: &InitData) -> Vec<InstanceData> {
        self.tiles
            .iter()
            .enumerate()
            .map(move |(index, tile)| self.tile_to_instance(index, tile, init_data))
            .collect::<Vec<_>>()
    }
}
