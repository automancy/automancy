pub mod state;

pub struct GameGui {
    pub renderer: YakuiWgpu,
    pub yak: Yakui,
    pub window: YakuiWinit,
    pub fonts: HashMap<String, Source>,
}

impl GameGui {
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
        let yak = Yakui::new();
        let renderer = yakui_wgpu::YakuiWgpu::new(device, queue);
        let window = yakui_winit::YakuiWinit::new(window);

        Self {
            renderer,
            yak,
            window,
            fonts: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameLoadResult {
    Loaded,
    LoadedMainMenu,
    Failed,
}

pub fn game_load_map_inner(state: &mut GameState, opt: LoadMapOption) -> GameLoadResult {
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

pub fn game_load_map(state: &mut GameState, map_name: String) -> GameLoadResult {
    game_load_map_inner(state, LoadMapOption::FromSave(map_name))
}
