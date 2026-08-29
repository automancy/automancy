use core::{fmt::Display, ops::Deref};
use std::{path::PathBuf, time::SystemTime};

use automancy_data::game::generic::DataMap;

use crate::actor::TileMap;

pub static MAP_PATH: &str = "map";
pub static MAP_EXT: &str = "zst";
pub static MAP_DATA_EXT: &str = "ron";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SaveFileName(String);

impl Display for SaveFileName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)?;

        Ok(())
    }
}

impl Deref for SaveFileName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for SaveFileName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl SaveFileName {
    pub fn new(str: &str) -> Self {
        const OPTIONS: sanitize_filename::Options = sanitize_filename::Options {
            truncate: true,
            windows: true,
            replacement: "",
        };

        let mut str = sanitize_filename::sanitize_with_options(str, OPTIONS);
        if str.is_empty() {
            str = "empty".to_string();
        }

        SaveFileName(str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GameMapId {
    Empty,
    SaveFile(SaveFileName),
    MainMenu,
    Debug,
}

impl Display for GameMapId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameMapId::Empty => f.write_str("<empty map>"),
            GameMapId::SaveFile(v) => f.write_fmt(format_args!("{v}")),
            GameMapId::MainMenu => f.write_str("<main menu map>"),
            GameMapId::Debug => f.write_str("<debug map>"),
        }
    }
}

/// Contains information about a map.
#[derive(Debug, Clone, Default)]
pub struct GameMapInfo {
    /// the last modified time.
    pub mtime: Option<SystemTime>,

    pub data: Box<DataMap>,
}

#[derive(Debug, Clone)]
pub struct GameMap {
    /// the id of the map
    pub id: GameMapId,

    pub tiles: TileMap,
    pub map_info: GameMapInfo,
}

impl GameMap {
    /// Creates a new empty map.
    pub fn new_empty(id: GameMapId) -> Self {
        Self {
            id,
            tiles: Default::default(),
            map_info: Default::default(),
        }
    }

    /// Gets the path to a map from its name.
    pub fn path(id: GameMapId) -> Option<PathBuf> {
        match id {
            GameMapId::SaveFile(map_name) => Some(PathBuf::from(MAP_PATH).join(map_name.as_ref())),
            _ => None,
        }
    }

    /// Gets the path to a map's data from its name.
    pub fn data(id: GameMapId) -> Option<PathBuf> {
        GameMap::path(id).map(|v| v.join("data").with_extension(MAP_DATA_EXT))
    }

    /// Gets the path to a map's tiles from its name.
    pub fn map(id: GameMapId) -> Option<PathBuf> {
        GameMap::path(id).map(|v| v.join("map").with_extension(MAP_EXT))
    }
}

pub mod serialize {
    use std::{fs, fs::File, io, io::Write, sync::Arc, time::SystemTime};

    use automancy_data::{
        game::{
            coord::TileCoord,
            generic::{
                DataMap,
                serailize::{DataMapRaw, IdMapping, IdMappingError},
            },
        },
        id::{Id, IdInterner, TileId},
    };
    use hashbrown::HashMap;
    use interpolator::Formattable;
    use serde::{Deserialize, Serialize};
    use thiserror::Error;

    use crate::{
        actor::{FlatTiles, message::TileMsg, util::multi_call_iter},
        persistent::{
            map::{GameMap, GameMapId, GameMapInfo},
            ron::ron_options,
        },
        resources::ResourceManager,
        state::error::ErrorManager,
    };

    static MAIN_MENU_MAP: &[u8] = include_bytes!("assets/main_menu/map.zst");
    static MAIN_MENU_MAP_DATA: &[u8] = include_bytes!("assets/main_menu/data.ron");

    const MAP_BUFFER_SIZE: usize = 256 * 1024;
    const MAP_DATA_BUFFER_SIZE: usize = 1024;

    #[derive(Debug, Error)]
    pub enum MapReadError {
        #[error("map file failed to decode")]
        MapFileDecodingError,
        #[error("map data file failed to decode")]
        MapDataFileDecodingError,
        #[error("map path is invalid")]
        InvalidPath,
        #[error("id_map couldn't be decoded: {0}")]
        IdMappingError(#[from] IdMappingError),
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct GameMapInfoRaw {
        /// The number of saved tiles.
        #[serde(default)]
        pub tile_count: u32,
        #[serde(default)]
        pub data: DataMapRaw,
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct GameMapRaw {
        tiles: Vec<(TileCoord, Id, DataMapRaw)>,
        id_map: IdMapping,
    }

    impl GameMapRaw {
        pub fn decode_data(&self, data: DataMapRaw, interner: &IdInterner) -> Result<DataMap, IdMappingError> {
            data.into_data(&self.id_map, interner)
        }

        pub fn encode_data(&mut self, data: DataMap, interner: &IdInterner) -> DataMapRaw {
            data.into_raw(&mut self.id_map, interner)
        }

        pub fn into_tiles(self, interner: &IdInterner) -> Result<FlatTiles, MapReadError> {
            let mut map = HashMap::new();

            for (coord, unmapped_id, data_raw) in self.tiles {
                let id = self.id_map.resolve(unmapped_id, interner)?;
                let mut data = Box::new(DataMap::new());

                for (unmapped_id, datum) in data_raw.into_inner() {
                    let id = self.id_map.resolve(unmapped_id, interner)?;
                    let datum = datum.into_datum(&self.id_map, interner)?;

                    data.set(id, datum);
                }

                map.insert(coord, (TileId(id), data));
            }

            Ok(map)
        }

        pub fn insert(&mut self, coord: TileCoord, id: TileId, data: DataMap, interner: &IdInterner) {
            self.id_map.insert(*id, interner);
            for id in data.keys() {
                self.id_map.insert(id, interner);
            }

            let data = data.into_raw(&mut self.id_map, interner);

            self.tiles.push((coord, *id, data));
        }
    }

    pub fn read_map_data(resource_man: &ResourceManager, map_id: GameMapId) -> anyhow::Result<(GameMapInfoRaw, Option<SystemTime>)> {
        let mut mtime = None;

        let decoded: ron::error::SpannedResult<GameMapInfoRaw> = match &map_id {
            GameMapId::Empty => Ok(GameMapInfoRaw::default()),
            GameMapId::SaveFile(name) => {
                log::debug!("Trying to read map metadata from {name}.");

                let Some(path) = GameMap::data(map_id.clone()) else {
                    return Err(MapReadError::InvalidPath.into());
                };

                let file = File::open(path)?;
                mtime = file.metadata().and_then(|v| v.modified().or(v.accessed())).ok();

                ron_options().from_reader(io::BufReader::with_capacity(MAP_DATA_BUFFER_SIZE, file))
            },
            GameMapId::MainMenu => ron_options().from_bytes(MAIN_MENU_MAP_DATA),
            GameMapId::Debug => panic!("debug map metadata cannot be read"),
        };

        match decoded {
            Ok(v) => Ok((v, mtime)),
            Err(e) => {
                log::error!("Error loading map '{map_id}', in reading map metadata: serde: {e}.");

                ErrorManager::push_err(
                    resource_man,
                    resource_man.registry.err_ids.invalid_map_data,
                    [("map_name", Formattable::display(&map_id)), ("error", Formattable::display(&e))],
                );

                Err(MapReadError::MapDataFileDecodingError.into())
            },
        }
    }

    pub fn read_map(resource_man: &ResourceManager, map_id: GameMapId) -> anyhow::Result<GameMapRaw> {
        let decoded: ron::error::SpannedResult<GameMapRaw> = match &map_id {
            GameMapId::Empty => Ok(GameMapRaw::default()),
            GameMapId::SaveFile(name) => {
                log::debug!("Trying to read map from {name}.");

                let Some(path) = GameMap::map(map_id.clone()) else {
                    return Err(MapReadError::InvalidPath.into());
                };

                let file = File::open(path)?;
                let decoder = zstd::Decoder::with_buffer(io::BufReader::with_capacity(MAP_BUFFER_SIZE, file)).unwrap();

                ron_options().from_reader(decoder)
            },
            GameMapId::MainMenu => ron_options().from_reader(zstd::Decoder::with_buffer(MAIN_MENU_MAP).unwrap()),
            GameMapId::Debug => panic!("debug map cannot be read"),
        };

        match decoded {
            Ok(v) => Ok(v),
            Err(e) => {
                log::error!("Error loading map '{map_id}', in reading map: serde: {e}.");

                ErrorManager::push_err(
                    resource_man,
                    resource_man.registry.err_ids.invalid_map_data,
                    [("map_name", Formattable::display(&map_id)), ("error", Formattable::display(&e))],
                );

                Err(MapReadError::MapFileDecodingError.into())
            },
        }
    }

    /// Loads a map from disk.
    pub fn load_map(resource_man: Arc<ResourceManager>, map_id: GameMapId) -> anyhow::Result<(FlatTiles, GameMapInfo)> {
        if matches!(map_id, GameMapId::SaveFile(..)) {
            let Some(path) = GameMap::path(map_id.clone()) else {
                return Err(MapReadError::InvalidPath.into());
            };
            fs::create_dir_all(path)?;
        }

        if GameMapId::Debug == map_id {
            let mut tiles = FlatTiles::default();

            for (idx, &tile) in resource_man.ordered_tiles.iter().enumerate() {
                if !tile.is_none() {
                    tiles.insert(TileCoord::new(idx as i32, 0), (tile, Box::new(DataMap::default())));
                }
            }

            return Ok((tiles, GameMapInfo::default()));
        }

        let (map_data_raw, mtime) = self::read_map_data(&resource_man, map_id.clone()).unwrap_or_default();
        let map_raw = self::read_map(&resource_man, map_id.clone()).unwrap_or_default();

        let map_info = GameMapInfo {
            mtime,
            data: Box::new(map_raw.decode_data(map_data_raw.data, &resource_man.interner)?),
        };
        let flat_tiles = map_raw.into_tiles(&resource_man.interner)?;

        Ok((flat_tiles, map_info))
    }

    /// Saves a map to disk.
    pub async fn save_map(map: &GameMap, interner: &IdInterner) -> io::Result<()> {
        // if [`GameMap::path`] returns Some, then data and map path must exist too
        if let Some(path) = GameMap::path(map.id.clone()) {
            fs::create_dir_all(path)?;

            let mut data_file = File::create(GameMap::data(map.id.clone()).unwrap()).unwrap();

            let map_file = File::create(GameMap::map(map.id.clone()).unwrap()).unwrap();
            let mut map_encoder = zstd::Encoder::new(map_file, 0).unwrap();

            let mut map_raw = GameMapRaw::default();
            {
                let mut tiles_data = multi_call_iter(
                    map.tiles.len(),
                    map.tiles.iter().map(|(coord, tile)| (*coord, tile.handle.clone())),
                    |_, reply| TileMsg::GetData(reply),
                    |k, v| (k, v),
                    None,
                )
                .await
                .unwrap();

                for (coord, tile) in map.tiles.iter() {
                    map_raw.insert(*coord, tile.id, *tiles_data.remove(coord).unwrap(), interner);
                }
            }

            ron_options()
                .to_io_writer(
                    &mut data_file,
                    &GameMapInfoRaw {
                        data: map_raw.encode_data(*map.map_info.data.clone(), interner),
                        tile_count: map.tiles.len() as u32,
                    },
                )
                .unwrap();
            ron_options().to_io_writer(&mut map_encoder, &map_raw).unwrap();

            data_file.flush().unwrap();
            map_encoder.finish().unwrap().flush().unwrap();

            log::info!("Saved map '{}'.", map.id);
        }

        Ok(())
    }
}
