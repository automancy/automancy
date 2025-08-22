use cosmic_text::fontdb::Source;
use hashbrown::HashMap;
use winit::window::Window;
use yakui::{Yakui, font::Fonts};
use yakui_wgpu::YakuiWgpu;
use yakui_winit::YakuiWinit;

pub mod custom;
pub struct AutomancyGui {
    pub renderer: YakuiWgpu,
    pub yak: Yakui,
    pub yakui_winit: YakuiWinit,
    pub fonts: HashMap<String, Source>,
}

impl AutomancyGui {
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

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, window: &Window) -> Self {
        let yak = Yakui::new();
        let renderer = yakui_wgpu::YakuiWgpu::new(device, queue);
        let window = yakui_winit::YakuiWinit::new(window);

        Self {
            renderer,
            yak,
            yakui_winit: window,
            fonts: Default::default(),
        }
    }
}
