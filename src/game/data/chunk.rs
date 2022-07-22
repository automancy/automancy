use std::{fs::File, path::Path};

use zstd::stream::{copy_decode, copy_encode};

use crate::game::data::raw::CanBeRaw;

use super::{
    data::{Data, RawData},
    grid::Grid,
    id::{Id, RawId},
    pos::Pos,
    raw::Raw,
    tile::{RawTile, Tile},
};

pub type Tiles = Grid<Tile, RawTile>;

pub type IdPool = Grid<Id, RawId>;
pub type DataPool = Grid<Data, RawData>;

#[derive(Debug)]
pub struct Chunk {
    pub chunk_pos: Pos,
    pub tiles: Tiles,
    pub id_pool: IdPool,
    pub data_pool: DataPool,
}

impl Chunk {
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

    fn load_tiles(pos: &Pos) -> Tiles {
        Self::load_part(Self::tiles_path(pos).as_str())
    }

    fn load_data_pool(pos: &Pos) -> DataPool {
        Self::load_part(Self::data_pool_path(pos).as_str())
    }

    fn load_id_pool(pos: &Pos) -> IdPool {
        Self::load_part(Self::id_pool_path(pos).as_str())
    }

    fn unload_part<T: CanBeRaw<R> + Clone + Default, R: Raw>(path: &str, data: Grid<T, R>) {
        let file = File::create(path).unwrap();

        let data: Vec<u8> = data.into();
        copy_encode(data.as_slice(), file, 0).unwrap();
    }

    fn unload_tiles(pos: &Pos, tiles: Tiles) {
        Self::unload_part(Self::tiles_path(pos).as_str(), tiles)
    }

    fn unload_data_pool(pos: &Pos, data_pool: DataPool) {
        Self::unload_part(Self::data_pool_path(pos).as_str(), data_pool)
    }

    fn unload_id_pool(pos: &Pos, id_pool: IdPool) {
        Self::unload_part(Self::id_pool_path(pos).as_str(), id_pool)
    }

    pub fn unload(self) {
        std::fs::create_dir_all(Path::new("map")).unwrap();

        Self::unload_tiles(&self.chunk_pos, self.tiles);
        Self::unload_data_pool(&self.chunk_pos, self.data_pool);
        Self::unload_id_pool(&self.chunk_pos, self.id_pool);
    }

    pub fn load(chunk_pos: Pos) -> Self {
        let tiles = Self::load_tiles(&chunk_pos);
        let id_pool = Self::load_id_pool(&chunk_pos);
        let data_pool = Self::load_data_pool(&chunk_pos);

        Self {
            chunk_pos,
            tiles,
            id_pool,
            data_pool,
        }
    }
}
