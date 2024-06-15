use std::time::Instant;
use std::{collections::BTreeMap, sync::Arc};

use automancy_defs::rendering::Vertex;
use gui::{Gui, GuiState};
use input::ActionType;
use ractor::ActorRef;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use automancy_defs::kira::manager::AudioManager;

use automancy_resources::types::font::Font;
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
    pub loop_store: EventLoopStorage,
    pub tokio: Runtime,
    pub game: ActorRef<GameSystemMessage>,
    pub camera: Camera,
    pub audio_man: AudioManager,
    pub start_instant: Instant,

    pub gui: Option<Gui>,
    pub renderer: Option<Renderer>,
    pub screenshotting: bool,

    pub logo: Option<ManagedTextureId>,
    pub input_hints: Vec<Vec<ActionType>>,
    pub puzzle_state: Option<(RhaiDataMap, bool)>,

    pub game_handle: Option<JoinHandle<()>>,

    pub vertices_init: Option<Vec<Vertex>>,
    pub indices_init: Option<Vec<u16>>,
    pub fonts_init: Option<BTreeMap<String, Font>>,
}
