use crate::game;
use crate::game::GameSystemMessage;
use crate::tile_entity::TileEntityMsg;
use automancy_defs::id::{Id, Interner};
use automancy_defs::{coord::TileCoord, id::TileId};
use automancy_resources::{
    data::{DataMap, DataMapRaw},
    error::push_err,
    format::Formattable,
};
use automancy_resources::{format::FormatContext, ResourceManager};
use hashbrown::HashMap;
use ractor::ActorRef;
use ron::error::SpannedResult;
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter};
use std::time::SystemTime;
use std::{fmt, fs::File};
use std::{fmt::Debug, io::Write};
use std::{fs, path::PathBuf};
use std::{io, sync::Arc};
use tokio::sync::Mutex;
use zstd::{Decoder, Encoder};

pub static MAP_PATH: &str = "map";
pub static MAP_EXT: &str = "zst";
pub static INFO_EXT: &str = "ron";

static MAIN_MENU_INFO: &[u8] = include_bytes!("assets/main_menu/info.ron");
static MAIN_MENU_MAP: &[u8] = include_bytes!("assets/main_menu/map.zst");

const INFO_BUFFER_SIZE: usize = 1024;
const MAP_BUFFER_SIZE: usize = 256 * 1024;

pub type Tiles = HashMap<TileCoord, TileId>;
pub type TileEntities = HashMap<TileCoord, ActorRef<TileEntityMsg>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadMapOption {
    FromSave(String),
    MainMenu,
    Debug, // TODO unused rn but can be useful to have a debug map
}

impl fmt::Display for LoadMapOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadMapOption::FromSave(v) => f.write_fmt(format_args!("Map {}", v)),
            LoadMapOption::MainMenu => f.write_str("<main menu>"),
            LoadMapOption::Debug => f.write_str("<debug map>"),
        }
    }
}

/// Contains information about a map.
#[derive(Debug, Clone, Default)]
pub struct MapInfo {
    /// The last save time as a UTC Unix timestamp.
    pub save_time: Option<SystemTime>,
    /// The map data.
    pub data: DataMap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapInfoRaw {
    /// The number of saved tiles.
    #[serde(default)]
    pub tile_count: u32,
    #[serde(default)]
    pub data: DataMapRaw,
}

/// A map stores tiles and tile entities to disk.
#[derive(Debug, Clone)]
pub struct GameMap {
    /// The name of the map, or a built-in map.
    /// Name should be sanitized.
    pub opt: LoadMapOption,
    /// The list of tiles.
    pub tiles: Tiles,
    /// The map's info.
    pub info: Arc<Mutex<MapInfo>>,
}

/// A map stores tiles and tile entities to disk.
#[derive(Debug, Serialize, Deserialize)]
pub struct MapRaw {
    pub tiles: Vec<(TileCoord, Id, DataMapRaw)>,
    pub tile_map: HashMap<Id, String>,
}

impl GameMap {
    /// Creates a new empty map.
    pub fn new_empty(opt: LoadMapOption) -> Self {
        Self {
            opt,
            tiles: Default::default(),
            info: Arc::new(Default::default()),
        }
    }

    /// Gets the path to a map from its name.
    pub fn path(opt: &LoadMapOption) -> Option<PathBuf> {
        match opt {
            LoadMapOption::FromSave(map_name) => Some(PathBuf::from(MAP_PATH).join(map_name)),
            _ => None,
        }
    }

    /// Gets the path to a map's info from its name.
    pub fn info(opt: &LoadMapOption) -> Option<PathBuf> {
        GameMap::path(opt).map(|v| v.join("info").with_extension(INFO_EXT))
    }

    /// Gets the path to a map's tiles from its name.
    pub fn map(opt: &LoadMapOption) -> Option<PathBuf> {
        GameMap::path(opt).map(|v| v.join("map").with_extension(MAP_EXT))
    }

    pub fn read_info(
        resource_man: &ResourceManager,
        opt: &LoadMapOption,
    ) -> Result<(MapInfoRaw, Option<SystemTime>), bool> {
        let mut time = None;

        let decoded: SpannedResult<MapInfoRaw> = match opt {
            LoadMapOption::FromSave(name) => {
                log::debug!("Trying to read map info from {name}");

                let path = Self::info(opt).unwrap();

                let file = File::open(path).map_err(|_| false)?;
                time = file
                    .metadata()
                    .and_then(|v| v.modified().or(v.accessed()))
                    .ok();

                ron::de::from_reader(BufReader::with_capacity(INFO_BUFFER_SIZE, file))
            }
            LoadMapOption::MainMenu => ron::de::from_bytes(MAIN_MENU_INFO),
            LoadMapOption::Debug => unreachable!(),
        };

        match decoded {
            Ok(v) => Ok((v, time)),
            Err(e) => {
                log::error!("Error loading map {opt}, in reading info: serde: {e:?}");

                push_err(
                    resource_man.registry.err_ids.invalid_map_data,
                    &FormatContext::from([("map_name", Formattable::display(&opt))].into_iter()),
                    resource_man,
                );

                Err(true)
            }
        }
    }

    pub fn read_map(resource_man: &ResourceManager, opt: &LoadMapOption) -> Result<MapRaw, bool> {
        let decoded: SpannedResult<MapRaw> = match opt {
            LoadMapOption::FromSave(name) => {
                log::debug!("Trying to read map data from {name}");

                let path = Self::map(opt).unwrap();

                let file = File::open(path).map_err(|_| false)?;
                let decoder =
                    Decoder::with_buffer(BufReader::with_capacity(MAP_BUFFER_SIZE, file)).unwrap();

                ron::de::from_reader(decoder)
            }
            LoadMapOption::MainMenu => {
                ron::de::from_reader(Decoder::with_buffer(MAIN_MENU_MAP).unwrap())
            }
            LoadMapOption::Debug => unreachable!(),
        };

        match decoded {
            Ok(v) => Ok(v),
            Err(e) => {
                log::error!("Error loading map {opt}, in reading map: serde: {e:?}");

                push_err(
                    resource_man.registry.err_ids.invalid_map_data,
                    &FormatContext::from([("map_name", Formattable::display(&opt))].into_iter()),
                    resource_man,
                );

                Err(true)
            }
        }
    }

    /// Loads a map from disk.
    pub async fn load(
        game: ActorRef<GameSystemMessage>,
        resource_man: Arc<ResourceManager>,
        opt: &LoadMapOption,
    ) -> Result<(Self, TileEntities), bool> {
        if let Some(path) = GameMap::path(opt) {
            fs::create_dir_all(path).map_err(|_| false)?;
        }

        let (info, save_time) = GameMap::read_info(&resource_man, opt)?;
        let map = GameMap::read_map(&resource_man, opt)?;

        let mut tiles = HashMap::new();
        let mut tile_entities = HashMap::new();

        for (coord, id, data) in map.tiles.into_iter() {
            if let Some(id) = map
                .tile_map
                .get(&id)
                .and_then(|id| resource_man.interner.get(id))
            {
                let tile_entity =
                    game::new_tile(resource_man.clone(), game.clone(), coord, TileId(id)).await;

                for (key, value) in data.to_data(&resource_man.interner) {
                    tile_entity
                        .send_message(TileEntityMsg::SetDataValue(key, value))
                        .unwrap();
                }

                tiles.insert(coord, TileId(id));
                tile_entities.insert(coord, tile_entity);
            }
        }

        Ok((
            Self {
                opt: opt.clone(),
                tiles,
                info: Arc::new(Mutex::new(MapInfo {
                    save_time,
                    data: info.data.to_data(&resource_man.interner),
                })),
            },
            tile_entities,
        ))
    }

    /// Saves a map to disk.
    pub async fn save(&self, interner: &Interner, tile_entities: &TileEntities) -> io::Result<()> {
        // if ::path returns Some, then info and map path must exist too
        if let Some(path) = GameMap::path(&self.opt) {
            fs::create_dir_all(path)?;

            let info = Self::info(&self.opt).unwrap();
            let info = File::create(info).unwrap();

            let mut info_writer = BufWriter::with_capacity(INFO_BUFFER_SIZE, info);

            let map = Self::map(&self.opt).unwrap();
            let map = File::create(map).unwrap();

            let map_writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, map);
            let mut map_encoder = Encoder::new(map_writer, 0).unwrap();

            let mut map_raw = MapRaw {
                tiles: vec![],
                tile_map: Default::default(),
            };

            for (coord, id) in self.tiles.iter() {
                if let Some(tile_entity) = tile_entities.get(coord) {
                    if !map_raw.tile_map.contains_key(&**id) {
                        map_raw
                            .tile_map
                            .insert(**id, interner.resolve(**id).unwrap().to_string());
                    }

                    let data = tile_entity
                        .call(TileEntityMsg::GetData, None)
                        .await
                        .unwrap()
                        .unwrap();
                    let data = data.to_raw(interner);

                    map_raw.tiles.push((*coord, **id, data));
                }
            }

            ron::ser::to_writer(
                &mut info_writer,
                &MapInfoRaw {
                    data: self.info.lock().await.data.to_raw(interner),
                    tile_count: self.tiles.len() as u32,
                },
            )
            .unwrap();

            ron::ser::to_writer(&mut map_encoder, &map_raw).unwrap();

            info_writer.flush().unwrap();
            map_encoder.do_finish().unwrap();

            log::info!("Saved map {}", self.opt);
        }

        Ok(())
    }
}

/// Sanitizes the name to ensure that the map can be used without problems on all platforms. This includes removing leading/trailing whitespace and periods, replacing non-alphanumeric characters, and replacing Windows disallowed names.
pub fn sanitize_name(name: String) -> String {
    if name.is_empty() {
        return "empty".to_string();
    }

    let name = name.trim();
    let name = name.trim_matches('.');
    name.replace(|c: char| !c.is_alphanumeric(), "_")
}
