use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::iter::Iterator;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use lazy_static::lazy_static;
use ractor::ActorRef;
use serde::{Deserialize, Serialize};
use zstd::{Decoder, Encoder};

use automancy_defs::coord::TileCoord;
use automancy_defs::id::{Id, Interner};
use automancy_defs::log;
use automancy_resources::chrono::{Local, Utc};
use automancy_resources::data::{DataMap, DataMapRaw};
use automancy_resources::ResourceManager;

use crate::game;
use crate::game::GameMsg;
use crate::tile_entity::{TileEntityMsg, TileModifier};

pub const MAP_PATH: &str = "map";
pub const MAP_EXT: &str = ".bin";
pub const MAIN_MENU: &str = ".mainmenu";

const MAP_BUFFER_SIZE: usize = 256 * 1024;

pub type Tiles = HashMap<TileCoord, (Id, TileModifier)>;
pub type TileEntities = HashMap<TileCoord, ActorRef<TileEntityMsg>>;

/// A map stores tiles and tile entities to disk.
#[derive(Debug, Clone)]
pub struct Map {
    /// The name of the map. Should be sanitized.
    pub map_name: String,
    /// The list of tiles.
    pub tiles: Tiles,
    /// The list of tile data.
    pub data: DataMap,
    /// The last save time as a UTC Unix timestamp.
    pub save_time: i64,
}

/// Contains information about a map to be saved to disk.
#[derive(Debug, Clone)]
pub struct MapInfo {
    pub map_name: String,
    pub tiles: usize,
    pub data: usize,
    pub save_time: i64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct MapHeader(Vec<(Id, String)>);

#[derive(Debug, Serialize, Deserialize)]
struct SerdeMap {
    #[serde(default)]
    pub header: MapHeader,
    #[serde(default)]
    pub serde_tiles: Vec<(TileCoord, SerdeTile)>,
    #[serde(default)]
    pub data: DataMapRaw,
    #[serde(default)]
    pub save_time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerdeTile(Id, TileModifier, DataMapRaw);

impl Map {
    /// Creates a new empty map.
    pub fn new_empty(map_name: String) -> Self {
        Self {
            map_name,

            tiles: Default::default(),
            data: Default::default(),
            save_time: Local::now().timestamp(),
        }
    }
    /// Gets the path to a map from its name.
    pub fn path(map_name: &str) -> PathBuf {
        PathBuf::from(format!("{MAP_PATH}/{map_name}{MAP_EXT}"))
    }
    /// Saves a map to disk.
    pub async fn save(&self, interner: &Interner, tile_entities: &TileEntities) {
        drop(fs::create_dir_all(MAP_PATH));

        let path = Self::path(&self.map_name);

        let file = File::create(path).unwrap();

        let writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, file);
        let mut encoder = Encoder::new(writer, 0).unwrap();

        let mut id_map = HashMap::new();
        let mut serde_tiles = Vec::new();
        for (coord, (id, tile_modifier)) in self.tiles.iter() {
            if let Some(tile_entity) = tile_entities.get(coord) {
                if !id_map.contains_key(id) {
                    id_map.insert(*id, interner.resolve(*id).unwrap().to_string());
                }

                let data = tile_entity
                    .call(TileEntityMsg::GetData, None)
                    .await
                    .unwrap()
                    .unwrap();
                let data = data.to_raw(interner);

                // tile_entity.stop(None);

                serde_tiles.push(Some((coord, SerdeTile(*id, *tile_modifier, data))));
            } else {
                serde_tiles.push(None);
            }
        }

        let header = MapHeader(id_map.into_iter().collect());

        let data = self.data.to_raw(interner);

        let save_time = Utc::now().timestamp();

        serde_json::to_writer(&mut encoder, &(header, serde_tiles, data, save_time)).unwrap();

        encoder.do_finish().unwrap();
    }
    /// Loads a map from disk.
    pub async fn load(
        game: ActorRef<GameMsg>,
        resource_man: &ResourceManager,
        map_name: String,
    ) -> (Self, TileEntities) {
        let path = Self::path(&map_name);

        let file = if let Ok(file) = File::open(path) {
            file
        } else {
            return (Map::new_empty(map_name), Default::default());
        };

        let reader = BufReader::with_capacity(MAP_BUFFER_SIZE, file);
        let decoder = Decoder::new(reader).unwrap();

        let decoded_map: serde_json::Result<SerdeMap> = serde_json::from_reader(decoder);

        if decoded_map.is_err() {
            log::error!("serde: {:?}", decoded_map.err());

            let err_map_name = format!("{}-ERR-{}", map_name, Local::now().format("%y%m%d%H%M%S"));

            resource_man.error_man.push(
                (
                    resource_man.registry.err_ids.invalid_map_data,
                    vec![map_name, err_map_name.clone()],
                ),
                resource_man,
            );
            return (Map::new_empty(err_map_name), Default::default());
        }
        let SerdeMap {
            header,
            serde_tiles,
            data,
            save_time,
            ..
        } = decoded_map.unwrap();

        let id_reverse = header.0.into_iter().collect::<HashMap<_, _>>();

        let mut tiles = HashMap::new();
        let mut tile_entities = HashMap::new();

        for (coord, SerdeTile(id, tile_modifier, data)) in serde_tiles.into_iter() {
            if let Some(id) = id_reverse
                .get(&id)
                .and_then(|id| resource_man.interner.get(id.as_str()))
            {
                let tile_entity = game::new_tile(game.clone(), coord, id, tile_modifier).await;
                let data = data.to_data(resource_man);

                data.0.into_iter().for_each(|(key, value)| {
                    tile_entity
                        .send_message(TileEntityMsg::SetData(key, value))
                        .unwrap();
                });

                tiles.insert(coord, (id, tile_modifier));
                tile_entities.insert(coord, tile_entity);
            }
        }

        let data = data.to_data(resource_man);

        (
            Self {
                map_name,

                tiles,
                data,

                save_time,
            },
            tile_entities,
        )
    }
    /// Sanitizes the name to ensure that the map can be used without problems on all platforms. This includes removing leading/trailing whitespace and periods, replacing non-alphanumeric characters, and replacing Windows disallowed names.
    pub fn sanitize_name(name: String) -> String {
        if name.is_empty() {
            return "empty".to_string();
        }
        let mut name = name;
        name = name.trim().to_string();
        name = name.trim_matches('.').to_string();
        name = name.replace(|c: char| !c.is_alphanumeric(), "_");

        if WIN_ILLEGAL_NAMES.contains(&name.to_ascii_uppercase().as_str()) {
            return format!("_{name}");
        }
        name
    }
}

lazy_static! {
    static ref WIN_ILLEGAL_NAMES: HashSet<&'static str> = HashSet::from([
        "CON", "PRN", "AUX", "CLOCK$", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
        "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
        "LPT9",
    ]);
}