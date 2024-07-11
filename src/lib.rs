use std::sync::Arc;
use std::time::Instant;

use automancy_defs::rendering::Vertex;
use gui::{Gui, GuiState};
use input::ActionType;
use options::MiscOptions;
use ractor::ActorRef;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use automancy_defs::kira::manager::AudioManager;

use automancy_resources::{data::DataMap, ResourceManager};

use yakui::{ManagedTextureId, Vec2};

use crate::camera::Camera;
use crate::event::EventLoopStorage;
use crate::game::GameSystemMessage;
use crate::input::InputHandler;
use crate::options::Options;
use crate::renderer::Renderer;

pub static VERSION: &str = env!("CARGO_PKG_VERSION");
pub static LOGO: &[u8] = include_bytes!("assets/logo.png");
pub static SSAO_NOISE_MAP: &[u8] = include_bytes!("assets/noise_map.png");

pub static SYMBOLS_FONT: &[u8] = include_bytes!("assets/SymbolsNerdFont-Regular.ttf");
pub static SYMBOLS_FONT_KEY: &str = "Symbols Nerd Font Mono";

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
    pub misc_options: MiscOptions,
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
    pub puzzle_state: Option<(DataMap, bool)>,

    pub game_handle: Option<JoinHandle<()>>,

    pub vertices_init: Option<Vec<Vertex>>,
    pub indices_init: Option<Vec<u16>>,
}

impl GameState {
    pub fn ui_viewport(&self) -> Vec2 {
        self.gui
            .as_ref()
            .unwrap()
            .yak
            .layout_dom()
            .viewport()
            .size()
    }
}
