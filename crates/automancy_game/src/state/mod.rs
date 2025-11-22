use std::{
    sync::Arc,
    time::{Instant, SystemTime},
};

use automancy_data::{
    game::{
        coord::TileCoord,
        generic::{DataMap, Datum},
    },
    id::{Id, TileId},
};
use hashbrown::HashMap;
use kira::AudioManager;
use ractor::{ActorRef, rpc::CallResult};
use tokio::{runtime::Runtime, sync::Mutex, task::JoinHandle};

use crate::{
    actor::{
        TileEntry,
        message::{GameMsg, PlaceTileResponse, TileMsg},
    },
    input::{ActionType, InputHandler, camera::GameCamera},
    persistent::{
        map,
        map::{GameMapData, GameMapId, serialize::GameMapDataRaw},
        options::{GameOptions, MiscOptions},
    },
    resources::{ResourceManager, types::item::ItemDef},
    state::ui::UiState,
};

pub mod error;
pub mod ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomancyGameLoadResult {
    Loaded,
    LoadedMainMenu,
    Failed,
}

/// Utility cache for game data that's used throughout runtime.
#[derive(Debug, Default)]
pub struct GameDataStorage {
    /// tag searching cache
    pub tag_cache: HashMap<Id, Arc<Vec<ItemDef>>>,

    pub map_info_cache: Vec<((GameMapDataRaw, Option<SystemTime>), String)>,
    pub loaded_map_info: Option<(GameMapId, GameMapData)>,

    // TODO are these useful anymore?
    // TODO pending removal after things get moved to scripting side more
    pub config_open_cache: Arc<Mutex<Option<ActorRef<TileMsg>>>>,
    pub pointing_cache: Arc<Mutex<Option<TileEntry>>>,

    pub input_hints: Vec<Vec<ActionType>>,
    pub puzzle_state: Option<(DataMap, bool)>,
}

pub struct AutomancyGameState {
    pub resource_man: Arc<ResourceManager>,
    pub audio_man: AudioManager,
    pub tokio: Runtime,

    pub ui_state: UiState,

    pub input_handler: InputHandler,
    pub camera: GameCamera,

    pub options: GameOptions,
    pub misc_options: MiscOptions,

    pub game_data: GameDataStorage,

    pub game_handle: ActorRef<GameMsg>,
    pub game_join_handle: Option<JoinHandle<()>>,

    pub start_instant: Instant,
}

impl AutomancyGameState {
    /// Refreshes the list of maps on the filesystem. Should be done every time the list of maps could have changed (on map creation/delete and on game load).
    pub fn refresh_map_info_cache(&mut self) {
        std::fs::create_dir_all(map::MAP_PATH).unwrap();

        let mut info_cache = std::fs::read_dir(map::MAP_PATH)
            .expect("map folder needs to exist and be readable")
            .flatten()
            .map(|f| f.file_name().to_str().unwrap().to_string())
            .filter(|f| !f.starts_with('.'))
            .flat_map(|name| {
                map::serialize::read_map_data(&self.resource_man, &GameMapId::SaveFile(name.clone()))
                    .ok()
                    .zip(Some(name))
            })
            .collect::<Vec<_>>();

        info_cache.sort_by(|a, b| a.1.cmp(&b.1));
        info_cache.sort_by(|a, b| a.0.1.unwrap_or(SystemTime::UNIX_EPOCH).cmp(&b.0.1.unwrap_or(SystemTime::UNIX_EPOCH)));
        info_cache.reverse();

        self.game_data.map_info_cache = info_cache;
    }

    /// Attempt to load the specified map, or load an empty map as fallback.
    pub fn load_map(&mut self, id: GameMapId) -> AutomancyGameLoadResult {
        let success = match self
            .tokio
            .block_on(self.game_handle.call(|reply| GameMsg::LoadMap(id.clone(), reply), None))
        {
            Ok(v) => v.unwrap(),
            Err(_) => false,
        };

        if success {
            self.game_data.loaded_map_info = self
                .tokio
                .block_on(self.game_handle.call(GameMsg::GetMapIdAndData, None))
                .unwrap()
                .unwrap();

            AutomancyGameLoadResult::Loaded
        } else if id != GameMapId::MainMenu {
            self.load_map(GameMapId::MainMenu)
        } else {
            log::warn!("Loading empty map as fallback.");
            self.load_map(GameMapId::Empty)
        }
    }

    pub fn link_tile(&mut self, id: Id, link_from: TileEntry, link_to: TileCoord) -> anyhow::Result<()> {
        let Ok(CallResult::Success(old)) = self.tokio.block_on(link_from.handle.call(|reply| TileMsg::GetDatum(id, reply), None)) else {
            return Ok(());
        };

        if old.is_some() {
            link_from.handle.send_message(TileMsg::RemoveDatum(id))?;

            self.audio_man.play(self.resource_man.audio["click"].clone())?;
            // TODO click2
        } else {
            link_from.handle.send_message(TileMsg::SetDatum(id, Datum::Coord(link_to)))?;

            self.audio_man.play(self.resource_man.audio["click"].clone())?;
        }

        Ok(())
    }

    pub fn place_tile(&mut self, id: TileId, coord: TileCoord) -> anyhow::Result<()> {
        let response = self
            .tokio
            .block_on(self.game_handle.call(
                |reply| GameMsg::PlaceTile {
                    coord,
                    tile: (id, DataMap::new()),
                    record: true,
                    reply: Some(reply),
                },
                None,
            ))?
            .unwrap();

        match response {
            PlaceTileResponse::Placed => {
                self.ui_state.config_open_at = Some(coord);
                self.ui_state.last_placed_at = Some(coord);

                self.audio_man.play(self.resource_man.audio["tile_placement"].clone())?;
            }
            PlaceTileResponse::Removed => {
                self.audio_man.play(self.resource_man.audio["tile_removal"].clone())?;
            }
            _ => {}
        }

        Ok(())
    }
}
