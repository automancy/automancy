use std::{fmt, fmt::Debug, path::PathBuf, time::SystemTime};

use automancy_data::game::generic::DataMap;

use crate::actor::TileMap;

pub static MAP_PATH: &str = "map";
pub static MAP_EXT: &str = "zst";
pub static MAP_DATA_EXT: &str = "ron";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GameMapId {
    Empty,
    SaveFile(String),
    MainMenu,
    Debug, // TODO unused rn but can be useful to have a debug map
}

impl fmt::Display for GameMapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
pub struct GameMapData {
    /// the last modified time.
    pub mtime: Option<SystemTime>,

    pub data: DataMap,
}

#[derive(Debug, Clone)]
pub struct GameMap {
    /// the id of the map
    pub id: GameMapId,

    pub tiles: TileMap,
    pub map_data: GameMapData,
}

impl GameMap {
    /// Creates a new empty map.
    pub fn new_empty(id: GameMapId) -> Self {
        Self {
            id,
            tiles: Default::default(),
            map_data: Default::default(),
        }
    }

    /// Gets the path to a map from its name.
    pub fn path(id: &GameMapId) -> Option<PathBuf> {
        match id {
            GameMapId::SaveFile(map_name) => Some(PathBuf::from(MAP_PATH).join(map_name)),
            _ => None,
        }
    }

    /// Gets the path to a map's data from its name.
    pub fn data(id: &GameMapId) -> Option<PathBuf> {
        GameMap::path(id).map(|v| v.join("data").with_extension(MAP_DATA_EXT))
    }

    /// Gets the path to a map's tiles from its name.
    pub fn map(id: &GameMapId) -> Option<PathBuf> {
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
                serailize::{DataMapRaw, IdMap, IdMapError},
            },
        },
        id::{Id, Interner, TileId},
    };
    use hashbrown::HashMap;
    use interpolator::Formattable;
    use serde::{Deserialize, Serialize};
    use thiserror::Error;

    use crate::{
        actor::{FlatTiles, message::TileMsg, util::multi_call_iter},
        persistent::{
            map::{GameMap, GameMapData, GameMapId},
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
        #[error("id_map did not contain String mapping for this Id: {0}")]
        MissingId(Id),
        #[error("map file failed to decode")]
        MapFileDecodingError,
        #[error("map data file failed to decode")]
        MapDataFileDecodingError,
    }

    impl MapReadError {
        fn id_map_err<T>(r: Result<T, IdMapError>) -> Result<T, MapReadError> {
            match r {
                Ok(v) => Ok(v),
                Err(err @ IdMapError::InternerMissingStringId(..)) => {
                    panic!(
                        "id_map should contain string mapping for every ID present in the map, it may be corrupted: {}",
                        err
                    );
                }
                Err(IdMapError::MapMissingId(id)) => Err(MapReadError::MissingId(id)),
            }
        }
    }

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct GameMapDataRaw {
        /// The number of saved tiles.
        #[serde(default)]
        pub tile_count: u32,
        #[serde(default)]
        pub data: DataMapRaw,
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct GameMapRaw {
        tiles: Vec<(TileCoord, Id, DataMapRaw)>,
        id_map: IdMap,
    }

    impl GameMapRaw {
        pub fn raw_to_data(&self, data: DataMapRaw, interner: &Interner) -> Result<DataMap, IdMapError> {
            data.into_data(&self.id_map, interner)
        }

        pub fn data_to_raw(&mut self, data: DataMap, interner: &Interner) -> DataMapRaw {
            data.into_raw(&mut self.id_map, interner)
        }

        pub fn into_tiles(self, interner: &Interner) -> Result<FlatTiles, MapReadError> {
            let mut map = HashMap::new();

            for (coord, unmapped_id, data_raw) in self.tiles {
                let id = MapReadError::id_map_err(self.id_map.resolve(unmapped_id, interner))?;
                let mut data = DataMap::new();

                for (unmapped_id, datum) in data_raw.into_inner() {
                    let id = MapReadError::id_map_err(self.id_map.resolve(unmapped_id, interner))?;
                    let datum = MapReadError::id_map_err(datum.into_datum(&self.id_map, interner))?;

                    data.set(id, datum);
                }

                map.insert(coord, (TileId(id), data));
            }

            Ok(map)
        }

        pub fn insert(&mut self, coord: TileCoord, id: TileId, data: DataMap, interner: &Interner) {
            self.id_map.insert(*id, interner);
            for id in data.keys() {
                self.id_map.insert(*id, interner);
            }

            let data = data.into_raw(&mut self.id_map, interner);

            self.tiles.push((coord, *id, data));
        }
    }

    pub fn read_map_data(resource_man: &ResourceManager, map_id: &GameMapId) -> anyhow::Result<(GameMapDataRaw, Option<SystemTime>)> {
        let mut mtime = None;

        let decoded: ron::error::SpannedResult<GameMapDataRaw> = match map_id {
            GameMapId::Empty => Ok(GameMapDataRaw::default()),
            GameMapId::SaveFile(name) => {
                log::debug!("Trying to read map metadata from {name}");

                let path = GameMap::data(map_id).unwrap();

                let file = File::open(path)?;
                mtime = file.metadata().and_then(|v| v.modified().or(v.accessed())).ok();

                ron::de::from_reader(io::BufReader::with_capacity(MAP_DATA_BUFFER_SIZE, file))
            }
            GameMapId::MainMenu => ron::de::from_bytes(MAIN_MENU_MAP_DATA),
            GameMapId::Debug => panic!("Debug map metadata cannot be read"),
        };

        match decoded {
            Ok(v) => Ok((v, mtime)),
            Err(e) => {
                log::error!("Error loading map {map_id}, in reading map metadata: serde: {e:?}");

                ErrorManager::push_err(
                    resource_man,
                    resource_man.registry.err_ids.invalid_map_data,
                    [("map_name", Formattable::display(&map_id))].into_iter(),
                );

                Err(MapReadError::MapDataFileDecodingError.into())
            }
        }
    }

    pub fn read_map(resource_man: &ResourceManager, map_id: &GameMapId) -> anyhow::Result<GameMapRaw> {
        let decoded: ron::error::SpannedResult<GameMapRaw> = match map_id {
            GameMapId::Empty => Ok(GameMapRaw::default()),
            GameMapId::SaveFile(name) => {
                log::debug!("Trying to read map from {name}");

                let path = GameMap::map(map_id).unwrap();

                let file = File::open(path)?;
                let decoder = zstd::Decoder::with_buffer(io::BufReader::with_capacity(MAP_BUFFER_SIZE, file)).unwrap();

                ron::de::from_reader(decoder)
            }
            GameMapId::MainMenu => ron::de::from_reader(zstd::Decoder::with_buffer(MAIN_MENU_MAP).unwrap()),
            GameMapId::Debug => panic!("Debug map cannot be read"),
        };

        match decoded {
            Ok(v) => Ok(v),
            Err(e) => {
                log::error!("Error loading map {map_id}, in reading map: serde: {e:?}");

                ErrorManager::push_err(
                    resource_man,
                    resource_man.registry.err_ids.invalid_map_data,
                    [("map_name", Formattable::display(&map_id))].into_iter(),
                );

                Err(MapReadError::MapFileDecodingError.into())
            }
        }
    }

    /// Loads a map from disk.
    pub fn load_map(resource_man: Arc<ResourceManager>, map_id: &GameMapId) -> anyhow::Result<(FlatTiles, GameMapData)> {
        if let Some(path) = GameMap::path(map_id) {
            fs::create_dir_all(path)?;
        }

        if GameMapId::Debug == *map_id {
            let mut tiles = FlatTiles::default();

            for (idx, &tile) in resource_man.ordered_tiles.iter().enumerate() {
                tiles.insert(TileCoord::new(idx as i32, 0), (tile, DataMap::default()));
            }

            return Ok((tiles, GameMapData::default()));
        }

        let (map_data_raw, mtime) = self::read_map_data(&resource_man, map_id)?;
        let map_raw = self::read_map(&resource_man, map_id)?;

        let map_data = GameMapData {
            mtime,
            data: map_raw.raw_to_data(map_data_raw.data, &resource_man.interner)?,
        };
        let flat_tiles = map_raw.into_tiles(&resource_man.interner)?;

        Ok((flat_tiles, map_data))
    }

    /// Saves a map to disk.
    pub async fn save_map(map: &GameMap, interner: &Interner) -> io::Result<()> {
        // if [`GameMap::path`] returns Some, then data and map path must exist too
        if let Some(path) = GameMap::path(&map.id) {
            fs::create_dir_all(path)?;

            let mut data_file = File::create(GameMap::data(&map.id).unwrap()).unwrap();

            let map_file = File::create(GameMap::map(&map.id).unwrap()).unwrap();
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
                    map_raw.insert(*coord, tile.id, tiles_data.remove(coord).unwrap(), interner);
                }
            }

            ron_options()
                .to_io_writer(
                    &mut data_file,
                    &GameMapDataRaw {
                        data: map_raw.data_to_raw(map.map_data.data.clone(), interner),
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
