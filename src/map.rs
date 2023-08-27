use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::iter::Iterator;
use std::sync::Arc;
use std::time::SystemTime;
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
use automancy_resources::chrono::Local;
use automancy_resources::data::{DataMap, DataMapRaw};
use automancy_resources::ResourceManager;

use crate::game;
use crate::game::GameMsg;
use crate::tile_entity::{TileEntityMsg, TileModifier};

pub const MAP_PATH: &str = "map";
pub const MAP_EXT: &str = ".zst";
pub const HEADER_EXT: &str = ".json";

pub const MAIN_MENU: &str = ".main_menu";

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
    pub save_time: Option<SystemTime>,
}

/// Contains information about a map.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MapInfo {
    /// The number of saved tiles.
    pub tile_count: u64,
    /// The last save time as a UTC Unix timestamp.
    pub save_time: Option<SystemTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerdeTile(Id, TileModifier, DataMapRaw);

#[derive(Debug, Serialize, Deserialize)]
pub struct MapHeader {
    #[serde(default)]
    pub tile_map: Vec<(Id, String)>,
    #[serde(default)]
    pub data: DataMapRaw,
    #[serde(default)]
    pub tile_count: u64,
}

impl Map {
    /// Creates a new empty map.
    pub fn new_empty(map_name: String) -> Self {
        Self {
            map_name,

            tiles: Default::default(),
            data: Default::default(),
            save_time: None,
        }
    }

    /// Gets the path to a map from its name.
    pub fn path(map_name: &str) -> PathBuf {
        PathBuf::from(format!("{MAP_PATH}/{map_name}/"))
    }

    /// Gets the path to a map's header from its name.
    pub fn header(map_name: &str) -> PathBuf {
        Map::path(map_name).join(format!("header{HEADER_EXT}"))
    }

    /// Gets the path to a map's tiles from its name.
    pub fn tiles(map_name: &str) -> PathBuf {
        Map::path(map_name).join(format!("tiles{MAP_EXT}"))
    }

    pub fn read_header(
        resource_man: &ResourceManager,
        map_name: &str,
    ) -> Option<(MapHeader, Option<SystemTime>)> {
        let path = Self::header(map_name);

        let file = File::open(path).ok()?;
        let time = file
            .metadata()
            .and_then(|v| v.modified().or(v.accessed()))
            .ok();

        let reader = BufReader::with_capacity(MAP_BUFFER_SIZE, file);

        let decoded: serde_json::Result<MapHeader> = serde_json::from_reader(reader);

        match decoded {
            Ok(v) => Some((v, time)),
            Err(e) => {
                log::error!("serde: {e:?}");

                let err_map_name =
                    format!("{}-ERR-{}", map_name, Local::now().format("%y%m%d%H%M%S"));

                resource_man.error_man.push(
                    (
                        resource_man.registry.err_ids.invalid_map_data,
                        vec![map_name.to_string(), err_map_name],
                    ),
                    resource_man,
                );

                None
            }
        }
    }

    pub fn read_tiles(
        resource_man: &ResourceManager,
        map_name: &str,
    ) -> Option<Vec<(TileCoord, SerdeTile)>> {
        let path = Self::tiles(map_name);

        let file = File::open(path).ok()?;
        let decoder = Decoder::new(file).unwrap();

        let decoded: serde_json::Result<Vec<(TileCoord, SerdeTile)>> =
            serde_json::from_reader(decoder);

        match decoded {
            Ok(v) => Some(v),
            Err(e) => {
                log::error!("serde: {e:?}");

                let err_map_name =
                    format!("{}-ERR-{}", map_name, Local::now().format("%y%m%d%H%M%S"));

                resource_man.error_man.push(
                    (
                        resource_man.registry.err_ids.invalid_map_data,
                        vec![map_name.to_string(), err_map_name],
                    ),
                    resource_man,
                );

                None
            }
        }
    }

    /// Loads a map from disk.
    pub async fn load(
        game: ActorRef<GameMsg>,
        resource_man: Arc<ResourceManager>,
        map_name: &str,
    ) -> (Self, TileEntities) {
        let Some((header, save_time)) = Map::read_header(&resource_man, map_name) else {
            return (Map::new_empty(map_name.to_string()), Default::default());
        };

        let Some(serde_tiles) = Map::read_tiles(&resource_man, map_name) else {
            return (Map::new_empty(map_name.to_string()), Default::default());
        };

        let id_reverse = header.tile_map.into_iter().collect::<HashMap<_, _>>();

        let mut tiles = HashMap::new();
        let mut tile_entities = HashMap::new();

        for (coord, SerdeTile(id, tile_modifier, data)) in serde_tiles.into_iter() {
            if let Some(id) = id_reverse
                .get(&id)
                .and_then(|id| resource_man.interner.get(id.as_str()))
            {
                let tile_entity =
                    game::new_tile(resource_man.clone(), game.clone(), coord, id, tile_modifier)
                        .await;
                let data = data.to_data(&resource_man).into_inner();

                for (key, value) in data {
                    tile_entity
                        .send_message(TileEntityMsg::SetDataValue(key, value))
                        .unwrap();
                }

                tiles.insert(coord, (id, tile_modifier));
                tile_entities.insert(coord, tile_entity);
            }
        }

        let data = header.data.to_data(&resource_man);

        (
            Self {
                map_name: map_name.to_string(),

                tiles,
                data,

                save_time,
            },
            tile_entities,
        )
    }

    /// Saves a map to disk.
    pub async fn save(&self, interner: &Interner, tile_entities: &TileEntities) {
        drop(fs::create_dir_all(Map::path(&self.map_name)));

        let header = Self::header(&self.map_name);
        let header = File::create(header).unwrap();

        let mut header_writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, header);

        let tiles = Self::tiles(&self.map_name);
        let tiles = File::create(tiles).unwrap();

        let tiles_writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, tiles);
        let mut tiles_encoder = Encoder::new(tiles_writer, 0).unwrap();

        let mut tile_map = HashMap::new();
        let mut serde_tiles = Vec::new();

        for (coord, (id, tile_modifier)) in self.tiles.iter() {
            if let Some(tile_entity) = tile_entities.get(coord) {
                if !tile_map.contains_key(id) {
                    tile_map.insert(*id, interner.resolve(*id).unwrap().to_string());
                }

                let data = tile_entity
                    .call(TileEntityMsg::GetData, None)
                    .await
                    .unwrap()
                    .unwrap();
                let data = data.to_raw(interner);

                serde_tiles.push((coord, SerdeTile(*id, *tile_modifier, data)));
            }
        }

        let mut tile_map = tile_map.into_iter().collect::<Vec<_>>();
        tile_map.sort_by_key(|v| v.0);

        let data = self.data.to_raw(interner);
        let tile_count = serde_tiles.len() as u64;

        serde_json::to_writer(
            &mut header_writer,
            &MapHeader {
                tile_map,
                data,
                tile_count,
            },
        )
        .unwrap();

        serde_json::to_writer(&mut tiles_encoder, &serde_tiles).unwrap();

        header_writer.flush().unwrap();
        tiles_encoder.do_finish().unwrap();
    }

    /// Sanitizes the name to ensure that the map can be used without problems on all platforms. This includes removing leading/trailing whitespace and periods, replacing non-alphanumeric characters, and replacing Windows disallowed names.
    pub fn sanitize_name(name: String) -> String {
        if name.is_empty() {
            return "empty".to_string();
        }
        let name = name.trim();
        let name = name.trim_matches('.');
        let name = name.replace(|c: char| !c.is_alphanumeric(), "_");

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
