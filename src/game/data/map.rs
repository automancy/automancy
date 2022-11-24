use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use zstd::{Decoder, Encoder};

use super::{
    id_pool::IdPool,
    tile::{Tile, TileCoord},
};

use serde::{Deserialize, Serialize};

// TODO chunk-ize the map again, but just for faster searching

#[derive(Debug, Serialize, Deserialize)]
pub struct Map {
    pub map_name: String,

    pub tiles: HashMap<TileCoord, Tile>,
    pub id_pool: IdPool,
}

pub const MAP_PATH: &str = "map";

const MAP_BUFFER_SIZE: usize = 128 * 1024;

impl Map {
    pub fn new_empty(map_name: String) -> Self {
        Self {
            map_name,
            tiles: HashMap::new(),
            id_pool: IdPool::new(),
        }
    }

    pub fn new(map_name: String, tiles: impl IntoIterator<Item = (TileCoord, Tile)>) -> Self {
        Self {
            map_name,
            tiles: HashMap::from_iter(tiles),
            id_pool: IdPool::new(),
        }
    }

    pub fn path(map_name: &str) -> PathBuf {
        PathBuf::from(format!("{}/{}.bin", MAP_PATH, map_name))
    }

    pub fn unload(self) {
        drop(std::fs::create_dir(MAP_PATH));

        let path = Self::path(&self.map_name);

        let file = File::create(path).unwrap();

        let writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, file);
        let mut encoder = Encoder::new(writer, 0).unwrap();

        serde_json::to_writer(&mut encoder, &self).unwrap();

        encoder.do_finish().unwrap();
    }

    pub fn load(map_name: String) -> Self {
        let path = Self::path(&map_name);

        let file = if let Ok(file) = File::open(path) {
            file
        } else {
            return Map::new_empty(map_name);
        };

        let reader = BufReader::with_capacity(MAP_BUFFER_SIZE, file);
        let decoder = Decoder::new(reader).unwrap();

        serde_json::from_reader(decoder).unwrap()
    }
}
