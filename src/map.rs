use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::sync::Arc;
use std::time::SystemTime;
use std::{fs, path::PathBuf};

use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use ractor::ActorRef;
use ron::error::SpannedResult;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use zstd::{Decoder, Encoder};

use automancy_defs::coord::TileCoord;
use automancy_defs::id::{Id, IdRaw, Interner};
use automancy_defs::log;
use automancy_resources::chrono::Local;
use automancy_resources::data::{DataMap, DataMapRaw};
use automancy_resources::ResourceManager;

use crate::game;
use crate::game::GameMsg;
use crate::tile_entity::TileEntityMsg;

pub const MAP_PATH: &str = "map";
pub const MAP_EXT: &str = ".zst";
pub const INFO_EXT: &str = ".ron";

pub const MAIN_MENU: &str = ".main_menu";

const MAP_BUFFER_SIZE: usize = 256 * 1024;

pub type Tiles = HashMap<TileCoord, Id>;
pub type TileEntities = HashMap<TileCoord, ActorRef<TileEntityMsg>>;

/// Contains information about a map.
#[derive(Debug, Clone, Default)]
pub struct MapInfo {
    /// The last save time as a UTC Unix timestamp.
    pub save_time: Option<SystemTime>,
    /// The map data.
    pub data: DataMap,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MapInfoRaw {
    /// The number of saved tiles.
    #[serde(default)]
    pub tile_count: u64,
    #[serde(default)]
    pub data: DataMapRaw,
}

/// A map stores tiles and tile entities to disk.
#[derive(Debug, Clone)]
pub struct Map {
    /// The name of the map. Should be sanitized.
    pub map_name: String,
    /// The list of tiles.
    pub tiles: Tiles,
    /// The map's info.
    pub info: Arc<Mutex<MapInfo>>,
}

/// A map stores tiles and tile entities to disk.
#[derive(Debug, Serialize, Deserialize)]
pub struct MapRaw {
    pub map_name: String,
    pub tiles: Vec<(TileCoord, Id, DataMapRaw)>,
    pub tile_map: HashMap<Id, IdRaw>,
}

impl Map {
    /// Creates a new empty map.
    pub fn new_empty(map_name: String) -> Self {
        Self {
            map_name,
            tiles: Default::default(),
            info: Arc::new(Default::default()),
        }
    }

    /// Gets the path to a map from its name.
    pub fn path(map_name: &str) -> PathBuf {
        PathBuf::from(format!("{MAP_PATH}/{map_name}/"))
    }

    /// Gets the path to a map's header from its name.
    pub fn info(map_name: &str) -> PathBuf {
        Map::path(map_name).join(format!("header{INFO_EXT}"))
    }

    /// Gets the path to a map's tiles from its name.
    pub fn map(map_name: &str) -> PathBuf {
        Map::path(map_name).join(format!("map{MAP_EXT}"))
    }

    pub fn read_info(
        resource_man: &ResourceManager,
        map_name: &str,
    ) -> Option<(MapInfoRaw, Option<SystemTime>)> {
        let path = Self::info(map_name);

        let file = File::open(path).ok()?;
        let time = file
            .metadata()
            .and_then(|v| v.modified().or(v.accessed()))
            .ok();

        let reader = BufReader::with_capacity(MAP_BUFFER_SIZE, file);

        let decoded: SpannedResult<MapInfoRaw> = ron::de::from_reader(reader);

        match decoded {
            Ok(v) => Some((v, time)),
            Err(e) => {
                log::error!("Serde: {e:?}");

                let err_map_name = format!(
                    "{}-ERR-{}",
                    map_name,
                    Local::now().format("%y-%m-%d-%H:%M:%S")
                );

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

    pub fn read_map(resource_man: &ResourceManager, map_name: &str) -> Option<MapRaw> {
        let path = Self::map(map_name);

        let file = File::open(path).ok()?;
        let decoder = Decoder::new(file).unwrap();

        let decoded: SpannedResult<MapRaw> = ron::de::from_reader(decoder);

        match decoded {
            Ok(v) => Some(v),
            Err(e) => {
                log::error!("Serde: {e:?}");

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
        let Some((info, save_time)) = Map::read_info(&resource_man, map_name) else {
            return (Map::new_empty(map_name.to_string()), Default::default());
        };

        let Some(map) = Map::read_map(&resource_man, map_name) else {
            return (Map::new_empty(map_name.to_string()), Default::default());
        };

        let mut tiles = HashMap::new();
        let mut tile_entities = HashMap::new();

        for (coord, id, data) in map.tiles.into_iter() {
            if let Some(id) = map
                .tile_map
                .get(&id)
                .and_then(|id| resource_man.interner.get(id.to_string()))
            {
                let tile_entity =
                    game::new_tile(resource_man.clone(), game.clone(), coord, id).await;
                let data = data.to_data(&resource_man.interner).into_inner();

                for (key, value) in data {
                    tile_entity
                        .send_message(TileEntityMsg::SetDataValue(key, value))
                        .unwrap();
                }

                tiles.insert(coord, id);
                tile_entities.insert(coord, tile_entity);
            }
        }

        (
            Self {
                map_name: map_name.to_string(),
                tiles,
                info: Arc::new(Mutex::new(MapInfo {
                    save_time,
                    data: info.data.to_data(&resource_man.interner),
                })),
            },
            tile_entities,
        )
    }

    /// Saves a map to disk.
    pub async fn save(&self, interner: &Interner, tile_entities: &TileEntities) {
        drop(fs::create_dir_all(Map::path(&self.map_name)));

        let info = Self::info(&self.map_name);
        let info = File::create(info).unwrap();

        let mut info_writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, info);

        let tiles = Self::map(&self.map_name);
        let tiles = File::create(tiles).unwrap();

        let tiles_writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, tiles);
        let mut tiles_encoder = Encoder::new(tiles_writer, 0).unwrap();

        let mut map_raw = MapRaw {
            map_name: self.map_name.clone(),
            tiles: vec![],
            tile_map: Default::default(),
        };

        for (coord, id) in self.tiles.iter() {
            if let Some(tile_entity) = tile_entities.get(coord) {
                if !map_raw.tile_map.contains_key(id) {
                    map_raw
                        .tile_map
                        .insert(*id, IdRaw::parse(interner.resolve(*id).unwrap()));
                }

                let data = tile_entity
                    .call(TileEntityMsg::GetData, None)
                    .await
                    .unwrap()
                    .unwrap();
                let data = data.to_raw(interner);

                map_raw.tiles.push((*coord, *id, data));
            }
        }

        ron::ser::to_writer(
            &mut info_writer,
            &MapInfoRaw {
                data: self.info.lock().await.data.to_raw(interner),
                tile_count: self.tiles.len() as u64,
            },
        )
        .unwrap();

        ron::ser::to_writer(&mut tiles_encoder, &map_raw).unwrap();

        info_writer.flush().unwrap();
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
