use automancy_defs::{id::Id, kira::manager::AudioManager, math::Vec2, rendering::Vertex};
use automancy_resources::{data::DataMap, types::item::ItemDef, ResourceManager};
use camera::GameCamera;
use cosmic_text::fontdb::Source;
use game::GameSystemMessage;
use hashbrown::HashMap;
use input::{ActionType, InputHandler};
use map::{LoadMapOption, MapInfo, MapInfoRaw};
use options::{GameOptions, MiscOptions};
use ractor::ActorRef;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant, SystemTime},
};
use tile_entity::{TileEntityMsg, TileEntityWithId};
use tokio::{runtime::Runtime, sync::Mutex, task::JoinHandle};
use ui_state::UiState;
use wgpu::{Device, Queue};
use winit::window::Window;
use yakui::{font::Fonts, ManagedTextureId, Yakui};
use yakui_wgpu::YakuiWgpu;
use yakui_winit::YakuiWinit;

pub mod camera;
pub mod game;
pub mod input;
pub mod map;
pub mod options;
pub mod tile_entity;
pub mod ui_state;
pub mod util;

pub struct GameGui<YakuiResources> {
    pub renderer: YakuiWgpu<YakuiResources>,
    pub yak: Yakui,
    pub window: YakuiWinit,
    pub fonts: HashMap<String, Source>,
}

impl<T> GameGui<T> {
    pub fn set_font(&mut self, symbols_font: &str, font: &str, font_source: Source) {
        let fonts = self.yak.dom().get_global_or_init(Fonts::default);

        log::info!("Setting font to {font}");

        fonts.load_font_source(font_source);

        fonts.set_sans_serif_family(font);
        fonts.set_serif_family(font);
        fonts.set_monospace_family(font);
        fonts.set_cursive_family(font);
        fonts.set_fantasy_family(font);

        fonts.load_font_source(self.fonts.get(symbols_font).unwrap().clone());
    }

    pub fn new(device: &Device, queue: &Queue, window: &Window) -> Self {
        let mut yak = Yakui::new();
        let renderer = yakui_wgpu::YakuiWgpu::new(&mut yak, device, queue);
        let window = yakui_winit::YakuiWinit::new(window);

        Self {
            renderer,
            yak,
            window,
            fonts: Default::default(),
        }
    }
}

/// Stores information that lives for the entire lifetime of the session, and is not dropped at the end of one event cycle or handled elsewhere.
#[derive(Debug, Default)]
pub struct EventLoopStorage {
    /// tag searching cache
    pub tag_cache: HashMap<Id, Arc<Vec<ItemDef>>>,
    /// the last frame's starting time
    pub frame_start: Option<Instant>,
    /// the elapsed time between each frame
    pub elapsed: Duration,

    pub map_infos_cache: Vec<((MapInfoRaw, Option<SystemTime>), String)>,
    pub map_info: Option<(Arc<Mutex<MapInfo>>, LoadMapOption)>,

    pub config_open_cache: Arc<Mutex<Option<ActorRef<TileEntityMsg>>>>,
    pub config_open_updating: Arc<AtomicBool>,
    pub pointing_cache: Arc<Mutex<Option<TileEntityWithId>>>,
    pub pointing_updating: Arc<AtomicBool>,
}

pub struct InnerGameState<YakuiResources, Renderer> {
    pub ui_state: UiState,
    pub options: GameOptions,
    pub misc_options: MiscOptions,
    pub resource_man: Arc<ResourceManager>,
    pub input_handler: InputHandler,
    pub loop_store: EventLoopStorage,
    pub tokio: Runtime,
    pub game: ActorRef<GameSystemMessage>,
    pub camera: GameCamera,
    pub audio_man: AudioManager,
    pub start_instant: Instant,

    pub gui: Option<GameGui<YakuiResources>>,
    pub renderer: Option<Renderer>,
    pub screenshotting: bool,

    pub logo: Option<ManagedTextureId>,
    pub input_hints: Vec<Vec<ActionType>>,
    pub puzzle_state: Option<(DataMap, bool)>,

    pub game_handle: Option<JoinHandle<()>>,

    pub vertices_init: Option<Vec<Vertex>>,
    pub indices_init: Option<Vec<u16>>,
}

impl<A, B> InnerGameState<A, B> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameLoadResult {
    Loaded,
    LoadedMainMenu,
    Failed,
}

pub fn game_load_map_inner<A, B>(
    state: &mut InnerGameState<A, B>,
    opt: LoadMapOption,
) -> GameLoadResult {
    let success = match state.tokio.block_on(
        state
            .game
            .call(|reply| GameSystemMessage::LoadMap(opt.clone(), reply), None),
    ) {
        Ok(v) => v.unwrap(),
        Err(_) => false,
    };

    if success {
        state.loop_store.map_info = state
            .tokio
            .block_on(state.game.call(GameSystemMessage::GetMapInfoAndName, None))
            .unwrap()
            .unwrap();

        GameLoadResult::Loaded
    } else if opt == LoadMapOption::MainMenu {
        GameLoadResult::Failed
    } else {
        game_load_map_inner(state, LoadMapOption::MainMenu)
    }
}

pub fn game_load_map<A, B>(state: &mut InnerGameState<A, B>, map_name: String) -> GameLoadResult {
    game_load_map_inner(state, LoadMapOption::FromSave(map_name))
}
