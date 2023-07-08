use egui::output::OpenUrl;
use egui::style::{WidgetVisuals, Widgets};
use egui::FontFamily::{Monospace, Proportional};
use egui::{
    Color32, Context, FontData, FontDefinitions, FontId, Response, Rounding, Stroke, Style,
    TextStyle, Ui, Visuals, Widget,
};
use egui_winit::State;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;

use winit::window::Window;

pub struct Gui {
    pub renderer: egui_wgpu::Renderer,
    pub context: Context,
    pub state: State,
    pub fonts: FontDefinitions,
}
/// Initialize the font families.
fn init_fonts(gui: &mut Gui) -> Result<(), Box<dyn Error>> {
    gui.fonts = FontDefinitions::default();
    let fonts_dir = fs::read_dir("resources/core/fonts")?;
    let font_files: Vec<(File, String)> = fonts_dir
        .filter_map(|f| f.ok())
        .map(|f| (File::open(f.path()), f))
        .filter(|(f, _d)| f.is_ok())
        .map(|(f, d)| (f.unwrap(), d.file_name().to_str().unwrap().to_string()))
        .collect();
    for (mut file, filename) in font_files {
        log::info!("loading font {filename}");
        let mut data: Vec<u8> = Vec::new();
        file.read_to_end(&mut data)?;
        gui.fonts
            .font_data
            .insert(filename.to_owned(), FontData::from_owned(data));
    }
    Ok(())
}
pub fn set_font(font: String, gui: &mut Gui) {
    gui.fonts
        .families
        .get_mut(&Proportional)
        .unwrap()
        .insert(0, font.to_owned());
    gui.fonts
        .families
        .get_mut(&Monospace)
        .unwrap()
        .insert(0, font);
    gui.context.set_fonts(gui.fonts.clone());
}

/// Initialize the GUI style.
fn init_styles(gui: &Gui) {
    gui.context.set_style(Style {
        override_text_style: None,
        override_font_id: None,
        text_styles: [
            (TextStyle::Small, FontId::new(9.0, Proportional)),
            (TextStyle::Body, FontId::new(13.0, Proportional)),
            (TextStyle::Button, FontId::new(13.0, Proportional)),
            (TextStyle::Heading, FontId::new(19.0, Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, Monospace)),
        ]
        .into(),
        wrap: None,
        visuals: Visuals {
            widgets: Widgets {
                noninteractive: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(248),
                    bg_fill: Color32::from_gray(170),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(160)), // separators, indentation lines
                    fg_stroke: Stroke::new(1.0, Color32::from_gray(80)),  // normal text color
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
                inactive: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(200), // button background
                    bg_fill: Color32::from_gray(200),      // checkbox background
                    bg_stroke: Default::default(),
                    fg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // button text
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
                hovered: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(220),
                    bg_fill: Color32::from_gray(190),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(105)), // e.g. hover over window edge or button
                    fg_stroke: Stroke::new(1.5, Color32::BLACK),
                    rounding: Rounding::same(3.0),
                    expansion: 1.0,
                },
                active: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(165),
                    bg_fill: Color32::from_gray(180),
                    bg_stroke: Stroke::new(1.0, Color32::BLACK),
                    fg_stroke: Stroke::new(2.0, Color32::BLACK),
                    rounding: Rounding::same(2.0),
                    expansion: 1.0,
                },
                open: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(220),
                    bg_fill: Color32::from_gray(210),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(160)),
                    fg_stroke: Stroke::new(1.0, Color32::BLACK),
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
            },
            ..Visuals::light()
        },
        ..Default::default()
    });
}

/// Initializes the GUI.
pub fn init_gui(renderer: egui_wgpu::Renderer, window: &Window) -> Gui {
    let mut gui = Gui {
        renderer,
        context: Default::default(),
        state: State::new(window),
        fonts: FontDefinitions::default(),
    };
    init_fonts(&mut gui).expect("Failed to initialize fonts");
    init_styles(&gui);

    gui
}

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct HyperlinkWidget<T: Widget> {
    url: String,
    widget: T,
}

impl<T: Widget> HyperlinkWidget<T> {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(widget: T, url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            widget,
        }
    }
}

impl<T: Widget> Widget for HyperlinkWidget<T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { url, widget } = self;

        let response = ui.add(widget);
        if response.clicked() {
            let modifiers = ui.ctx().input(|i| i.modifiers);
            ui.ctx().output_mut(|o| {
                o.open_url = Some(OpenUrl {
                    url: url.clone(),
                    new_tab: modifiers.any(),
                });
            });
        }
        if response.middle_clicked() {
            ui.ctx().output_mut(|o| {
                o.open_url = Some(OpenUrl {
                    url: url.clone(),
                    new_tab: true,
                });
            });
        }
        response.on_hover_text(url)
    }
}
