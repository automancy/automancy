use std::{
    sync::Arc,
    time::{Instant, SystemTime},
};

use automancy_data::{
    async_util::AsyncSwapBuf,
    game::{
        coord::TileCoord,
        generic::{DataMap, Datum, DatumChange},
    },
    id::Id,
};
use ractor::ActorRef;
use tokio::{runtime::Runtime, task::JoinHandle};

use crate::{
    actor::{TileEntry, message::GameMsg},
    input::{InputHandler, camera::GameCamera},
    persistent::{
        map,
        map::{GameMapId, GameMapInfo, SaveFileName, serialize::GameMapInfoRaw},
        options::{GameOptions, MiscOptions},
    },
    resources::ResourceManager,
    script::UiElement,
};

pub mod error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomancyGameLoadResult {
    Loaded,
    LoadedMainMenu,
    Failed,
}

/// Utility cache for game data that's used throughout runtime.
#[derive(Debug, Default)]
pub struct GameDataStorage {
    pub map_infos: Vec<(String, (GameMapInfoRaw, Option<SystemTime>))>,

    pub loaded_map: AsyncSwapBuf<Option<(GameMapId, GameMapInfo)>>,
    pub config_open: AsyncSwapBuf<Option<(TileEntry, Box<DataMap>, UiElement)>>,
    pub pointing_at: AsyncSwapBuf<Option<TileEntry>>,

    /// Note: this field is read-only, modifying this field has no effect to the game.
    ///
    /// If you want to modify it, please use [`GameDataStorage::set_map_datum`], [`GameDataStorage::add_map_datum`], or [`GameDataStorage::sub_map_datum`].
    pub map_data: Box<DataMap>,
    map_data_buf: AsyncSwapBuf<Box<DataMap>>,
    map_data_changes: Vec<DatumChange>,
}

impl GameDataStorage {
    pub fn refresh_async_data(&mut self, game_state: &mut AutomancyGameState) {
        use ractor::rpc::CallResult;

        use crate::actor::message::TileMsg;

        self.loaded_map.update_with_runtime(&game_state.tokio, || {
            let game_handle = game_state.game_handle.clone();

            async move {
                if let Ok(CallResult::Success(Some(map))) = game_handle.call(GameMsg::GetMap, None).await {
                    Some(map)
                } else {
                    None
                }
            }
        });

        self.config_open.update_with_runtime(&game_state.tokio, || {
            let game_handle = game_state.game_handle.clone();
            let config_open_at = game_state.config_open_at;

            async move {
                let config_open_at = config_open_at?;

                let Ok(CallResult::Success(Some(tile))) = game_handle.call(|reply| GameMsg::GetTile(config_open_at, reply), None).await
                else {
                    return None;
                };

                let Ok(CallResult::Success(data)) = tile.handle.call(TileMsg::GetData, None).await else {
                    return None;
                };

                let Ok(CallResult::Success(Some(tile_config_ui))) = tile.handle.call(TileMsg::GetTileConfigUi, None).await else {
                    return None;
                };

                Some((tile, data, tile_config_ui))
            }
        });

        self.pointing_at.update_with_runtime(&game_state.tokio, || {
            let game_handle = game_state.game_handle.clone();
            let pointing_at = game_state.camera.cursor_coord;

            async move {
                let Ok(CallResult::Success(Some(tile))) = game_handle.call(|reply| GameMsg::GetTile(pointing_at, reply), None).await else {
                    return None;
                };

                Some(tile)
            }
        });

        let map_data_update = || {
            let game_handle = game_state.game_handle.clone();

            async move {
                if let Ok(CallResult::Success(data)) = game_handle.call(GameMsg::GetMapData, None).await {
                    data
                } else {
                    Box::new(DataMap::new())
                }
            }
        };

        if !self.map_data_changes.is_empty() {
            game_state.tokio.block_on(async {
                game_state
                    .game_handle
                    .call(
                        |reply| GameMsg::ChangeMapData(std::mem::take(&mut self.map_data_changes), reply),
                        None,
                    )
                    .await
                    .unwrap()
                    .unwrap();

                if let Some(handle) = self.map_data_buf.update_with_runtime(&game_state.tokio, map_data_update) {
                    handle.await.unwrap();
                }
            });
        } else {
            self.map_data_buf.update_with_runtime(&game_state.tokio, map_data_update);
        }

        self.map_data = self.map_data_buf.read_latest().clone();
    }

    #[inline]
    pub fn set_map_datum(&mut self, id: Id, datum: Datum) {
        let change = DatumChange::Set(id, datum);

        self.map_data.handle_change(change.clone());
        self.map_data_changes.push(change);
    }

    #[inline]
    pub fn add_map_datum(&mut self, id: Id, datum: Datum) {
        let change = DatumChange::Add(id, datum);

        self.map_data.handle_change(change.clone());
        self.map_data_changes.push(change);
    }

    #[inline]
    pub fn sub_map_datum(&mut self, id: Id, datum: Datum) {
        let change = DatumChange::Sub(id, datum);

        self.map_data.handle_change(change.clone());
        self.map_data_changes.push(change);
    }
}

pub struct AutomancyGameState {
    pub resource_man: Arc<ResourceManager>,

    #[cfg(not(miri))]
    pub audio_man: kira::AudioManager,
    #[cfg(miri)]
    pub audio_man: crate::miri::StubbedAudioManager,

    pub tokio: Runtime,

    pub input_handler: InputHandler,
    pub camera: GameCamera,
    /// the tile that has its config menu open.
    pub config_open_at: Option<TileCoord>,

    pub options: GameOptions,
    pub misc_options: MiscOptions,

    pub game_handle: ActorRef<GameMsg>,
    pub game_join_handle: Option<JoinHandle<()>>,

    pub game_start: Instant,
}

impl AutomancyGameState {
    /// Refreshes the list of maps on the filesystem. Should be done every time the list of maps could have changed (on map creation/delete and on game load).
    pub fn refresh_maps_cache(&mut self, game_data: &mut GameDataStorage) {
        std::fs::create_dir_all(map::MAP_PATH).unwrap();

        let mut map_infos = std::fs::read_dir(map::MAP_PATH)
            .expect("map folder needs to exist and be readable")
            .flatten()
            .map(|f| f.file_name().to_str().unwrap().to_string())
            .filter(|f| !f.starts_with('.'))
            .flat_map(|name| {
                let data = map::serialize::read_map_data(&self.resource_man, GameMapId::SaveFile(SaveFileName::new(&name))).ok();

                Some(name).zip(data)
            })
            .collect::<Vec<_>>();

        map_infos.sort_by(|a, b| a.0.cmp(&b.0));
        map_infos.sort_by(|a, b| {
            a.1.1
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .cmp(&b.1.1.unwrap_or(SystemTime::UNIX_EPOCH))
        });
        map_infos.reverse();

        game_data.map_infos = map_infos;
    }

    /// Attempt to load the specified map, or load an empty map as fallback.
    pub fn load_map(&mut self, game_data: &mut GameDataStorage, id: GameMapId) -> AutomancyGameLoadResult {
        let success = match self
            .tokio
            .block_on(self.game_handle.call(|reply| GameMsg::LoadMap(id.clone(), reply), None))
        {
            Ok(v) => v.unwrap(),
            Err(_) => false,
        };

        if success {
            if let Some(handle) = game_data.loaded_map.update_with_runtime(&self.tokio, || {
                let game_handle = self.game_handle.clone();

                async move {
                    let map = game_handle.call(GameMsg::GetMap, None).await;
                    map.unwrap().unwrap()
                }
            }) {
                self.tokio.block_on(handle).unwrap();
            }

            AutomancyGameLoadResult::Loaded
        } else if id != GameMapId::MainMenu {
            log::warn!("Loading map '{id}' failed! Falling back to main menu instead.");
            self.load_map(game_data, GameMapId::MainMenu)
        } else if id == GameMapId::MainMenu {
            log::warn!("I can't even load the main menu!? Loading an empty map as fallback.");
            self.load_map(game_data, GameMapId::Empty)
        } else {
            panic!("empty map should be loadable no matter what")
        }
    }
}
