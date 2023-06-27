use fuse_rust::Fuse;

use automancy::gpu::Gpu;
use automancy_defs::cg::{DPoint2, Double, Float};
use automancy_defs::cgmath::MetricSpace;
use automancy_defs::egui::epaint::Shadow;
use automancy_defs::egui::style::{WidgetVisuals, Widgets};
use automancy_defs::egui::FontFamily::{Monospace, Proportional};
use automancy_defs::egui::{
    Color32, FontData, FontDefinitions, FontId, Frame, Rgba, Rounding, ScrollArea, Stroke, Style,
    TextStyle, Ui, Visuals,
};
use automancy_defs::egui_winit_vulkano::{Gui, GuiConfig};
use automancy_defs::id::Id;
use automancy_defs::rendering::GameVertex;
use automancy_defs::vulkano::image::SampleCount::Sample4;
use automancy_defs::winit::event_loop::EventLoop;
use automancy_defs::{cgmath, colors};
use automancy_resources::ResourceManager;

use crate::IOSEVKA_FONT;

const MARGIN: Float = 8.0;
const ITEM_ICON_SIZE: Float = 32.0;

pub mod debug;
pub mod error;
pub mod item;
pub mod menu;
pub mod popup;
pub mod tile_config;
pub mod tile_info;
pub mod tile_selection;

/// The state of the main game GUI.
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum GuiState {
    MainMenu,
    MapLoad,
    Options,
    Ingame,
    Paused,
}

/// The state of popups (which are on top of the main GUI), if any should be displayed.
#[derive(Eq, PartialEq, Clone)]
pub enum PopupState {
    None,
    MapCreate,
    MapDeleteConfirmation(String),
    InvalidName,
}

/// Initialize the font families.
fn init_fonts(gui: &Gui) {
    let mut fonts = FontDefinitions::default();
    let iosevka = "iosevka";

    fonts
        .font_data
        .insert(iosevka.to_owned(), FontData::from_static(IOSEVKA_FONT));

    fonts
        .families
        .get_mut(&Proportional)
        .unwrap()
        .insert(0, iosevka.to_owned());
    fonts
        .families
        .get_mut(&Monospace)
        .unwrap()
        .insert(0, iosevka.to_owned());

    gui.context().set_fonts(fonts);
}

/// Initialize the GUI style.
fn init_styles(gui: &Gui) {
    gui.context().set_style(Style {
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
pub fn init_gui(event_loop: &EventLoop<()>, gpu: &Gpu) -> Gui {
    let gui = Gui::new_with_subpass(
        event_loop,
        gpu.surface.clone(),
        gpu.queue.clone(),
        gpu.gui_subpass.clone(),
        GuiConfig {
            preferred_format: Some(gpu.alloc.swapchain.image_format()),
            is_overlay: true,
            samples: Sample4,
        },
    );

    init_fonts(&gui);
    init_styles(&gui);

    gui
}

/// Creates a default frame.
pub fn default_frame() -> Frame {
    Frame::none()
        .fill(colors::WHITE.multiply(0.65).into())
        .shadow(Shadow {
            extrusion: 8.0,
            color: colors::DARK_GRAY.multiply(0.5).into(),
        })
        .rounding(Rounding::same(5.0))
}

/// Produces a line shape.
pub fn make_line(a: DPoint2, b: DPoint2, w: Double, color: Rgba) -> [GameVertex; 6] {
    let v = b - a;
    let l = a.distance(b) * 16.0;
    let t = cgmath::vec2(-v.y / l, v.x / l);
    let t = t / w;

    let a0 = (a + t).cast::<Float>().unwrap();
    let a1 = (a - t).cast::<Float>().unwrap();
    let b0 = (b + t).cast::<Float>().unwrap();
    let b1 = (b - t).cast::<Float>().unwrap();

    let a = GameVertex {
        pos: [a0.x, a0.y, 0.0],
        color: color.to_array(),
        normal: [0.0, 0.0, 0.0],
    };
    let b = GameVertex {
        pos: [b0.x, b0.y, 0.0],
        color: color.to_array(),
        normal: [0.0, 0.0, 0.0],
    };
    let c = GameVertex {
        pos: [a1.x, a1.y, 0.0],
        color: color.to_array(),
        normal: [0.0, 0.0, 0.0],
    };
    let d = GameVertex {
        pos: [b1.x, b1.y, 0.0],
        color: color.to_array(),
        normal: [0.0, 0.0, 0.0],
    };

    [a, b, c, b, c, d]
}

/// Draws a search bar.
pub fn searchable_id<'a>(
    ui: &mut Ui,
    resource_man: &'a ResourceManager,
    fuse: &Fuse,
    ids: &[Id],
    new_id: &mut Option<Id>,
    filter: &mut String,
    name: &'static impl Fn(&'a ResourceManager, &Id) -> &'a str,
) {
    ui.text_edit_singleline(filter);

    ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
        ui.set_width(ui.available_width());

        let ids = if !filter.is_empty() {
            let mut filtered = ids
                .iter()
                .flat_map(|id| {
                    let result = fuse.search_text_in_string(filter, name(resource_man, id));
                    let score = result.map(|v| v.score);

                    if score.unwrap_or(0.0) > 0.4 {
                        None
                    } else {
                        Some(*id).zip(score)
                    }
                })
                .collect::<Vec<_>>();

            filtered.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

            filtered.into_iter().map(|v| v.0).collect::<Vec<_>>()
        } else {
            ids.to_vec()
        };

        for id in ids {
            ui.radio_value(new_id, Some(id), name(resource_man, &id));
        }
    });
}
