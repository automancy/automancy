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

pub struct InnerGameState {
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

    pub gui: Option<GameGui>,
    pub renderer: Option<Renderer>,
    pub screenshotting: bool,

    pub logo: Option<ManagedTextureId>,
    pub input_hints: Vec<Vec<ActionType>>,
    pub puzzle_state: Option<(DataMap, bool)>,

    pub game_handle: Option<JoinHandle<()>>,

    pub vertices_init: Option<Vec<GpuVertex>>,
    pub indices_init: Option<Vec<u16>>,
}
