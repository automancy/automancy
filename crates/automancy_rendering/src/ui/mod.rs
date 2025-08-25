use hashbrown::HashMap;
use winit::window::Window;
use yakui::{Yakui, font::Fonts};
use yakui_wgpu::YakuiWgpu;
use yakui_winit::YakuiWinit;

pub mod custom;

#[derive(Debug)]
pub struct UiRenderer {
    pub gui_resources: GuiResources,
    pub objects: HashMap<UserPaintCallId, RenderObject>,
    pub object_id_map: HashMap<RenderObjectDiscriminants, RangeSetBlaze<UserPaintCallId>>,
}

impl UiRenderer {
    pub fn new(gui_resources: GuiResources) -> Self {
        Self {
            gui_resources,
            objects: Default::default(),
            object_id_map: Default::default(),
        }
    }

    pub fn get_objects_of(
        &self,
        ty: RenderObjectDiscriminants,
    ) -> HashMap<UserPaintCallId, RenderObject> {
        let ranges = self.object_id_map.get(ty).unwrap_or_default();

        ranges
            .iter()
            .flat_map(|id| self.objects.get(id).map(|v| (id, v)))
            .collect()
    }

    pub fn start_render(&mut self) {
        if automancy_ui::custom::should_rerender() {
            let objects = automancy_ui::custom::take_objects();

            let mut object_id_map = HashMap::new();
            for (id, object) in &objects {
                let ty = RenderObjectDiscriminants::from(object);

                if !object_id_map.contains_key(&ty) {
                    object_id_map.insert(ty, RangeSetBlaze::new());
                }
                object_id_map.get_mut(&ty).unwrap().insert(*id);
            }

            self.object_id_map = object_id_map;
            self.objects = objects;
        }
    }
}

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
