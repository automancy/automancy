use std::{sync::Arc, time::Instant};

use automancy_data::game::generic::DataMap;
use kira::AudioManager;
use ractor::ActorRef;
use tokio::{runtime::Runtime, task::JoinHandle};

use crate::{
    actor::{map::GameMapId, message::GameMsg},
    input::{
        camera::GameCamera,
        handler::{ActionType, InputHandler},
    },
    persistent::options::{GameOptions, MiscOptions},
    resources::ResourceManager,
    state::event::EventLoopStorage,
};

pub mod event;
pub mod ui;

pub struct AutomancyGameState {
    pub resource_man: Arc<ResourceManager>,
    pub loop_store: EventLoopStorage,
    pub input_handler: InputHandler,
    pub audio_man: AudioManager,
    pub camera: GameCamera,

    pub tokio: Runtime,
    pub game: ActorRef<GameMsg>,
    pub game_handle: Option<JoinHandle<()>>,

    pub options: GameOptions,
    pub misc_options: MiscOptions,

    pub start_instant: Instant,

    pub input_hints: Vec<Vec<ActionType>>,
    pub puzzle_state: Option<(DataMap, bool)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomancyGameLoadResult {
    Loaded,
    LoadedMainMenu,
    Failed,
}

pub fn game_load_map_inner(
    state: &mut AutomancyGameState,
    id: GameMapId,
) -> AutomancyGameLoadResult {
    let success = match state.tokio.block_on(
        state
            .game
            .call(|reply| GameMsg::LoadMap(id.clone(), reply), None),
    ) {
        Ok(v) => v.unwrap(),
        Err(_) => false,
    };

    if success {
        state.loop_store.map_info = state
            .tokio
            .block_on(state.game.call(GameMsg::GetMapIdAndData, None))
            .unwrap()
            .unwrap();

        AutomancyGameLoadResult::Loaded
    } else if id == GameMapId::MainMenu {
        AutomancyGameLoadResult::Failed
    } else {
        game_load_map_inner(state, GameMapId::MainMenu)
    }
}

pub fn game_load_map(state: &mut AutomancyGameState, map_name: String) -> AutomancyGameLoadResult {
    game_load_map_inner(state, GameMapId::SaveFile(map_name))
}
