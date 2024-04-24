use std::sync::Arc;
use std::time::Instant;

use gui::{Gui, GuiState};
use input::{ActionType, KeyAction};
use ractor::ActorRef;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use automancy_resources::kira::manager::AudioManager;
use automancy_resources::types::function::RhaiDataMap;
use automancy_resources::ResourceManager;
use yakui::ManagedTextureId;

use crate::camera::Camera;
use crate::event::EventLoopStorage;
use crate::game::GameSystemMessage;
use crate::input::InputHandler;
use crate::options::Options;
use crate::renderer::Renderer;

pub static VERSION: &str = env!("CARGO_PKG_VERSION");
pub static LOGO: &[u8] = include_bytes!("assets/logo.png");
pub static SSAO_NOISE_MAP: &[u8] = include_bytes!("assets/noise_map.png");

pub mod camera;
pub mod event;
pub mod game;
pub mod gpu;
pub mod gui;
pub mod input;
pub mod map;
pub mod options;
pub mod renderer;
pub mod tile_entity;
pub mod util;

pub struct GameState {
    pub gui_state: GuiState,
    pub options: Options,
    pub resource_man: Arc<ResourceManager>,
    pub input_handler: InputHandler,
    pub input_hints: Vec<Vec<ActionType>>,
    pub camera: Camera,
    pub loop_store: EventLoopStorage,
    pub tokio: Runtime,
    pub game: ActorRef<GameSystemMessage>,
    pub gui: Gui,
    pub audio_man: AudioManager,
    pub start_instant: Instant,
    pub renderer: Renderer<'static>,
    pub game_handle: Option<JoinHandle<()>>,
    pub puzzle_state: Option<(RhaiDataMap, bool)>,
    pub logo: ManagedTextureId,
}
