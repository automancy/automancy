use std::{sync::Arc, time::Instant};

use automancy_data::game::generic::DataMap;
use kira::AudioManager;
use ractor::ActorRef;
use tokio::{runtime::Runtime, task::JoinHandle};

use crate::{
    actor::message::GameMsg,
    input::{
        camera::GameCamera,
        handler::{ActionType, InputHandler},
    },
    persistent::options::{GameOptions, MiscOptions},
    resources::ResourceManager,
    state::{event::EventLoopStorage, ui::UiState},
};

pub mod event;
pub mod ui;

pub struct AutomancyGameState {
    pub resource_man: Arc<ResourceManager>,
    pub loop_store: EventLoopStorage,
    pub ui_state: UiState,
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
