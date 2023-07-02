use egui::epaint::Shadow;
use egui::{Frame, Rgba, Rounding, ScrollArea, Ui};
use fuse_rust::Fuse;

use automancy_defs::cgmath::MetricSpace;
use automancy_defs::id::Id;
use automancy_defs::math::{DPoint2, Double, Float};
use automancy_defs::rendering::Vertex;
use automancy_defs::{cgmath, colors};
use automancy_resources::ResourceManager;

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
pub fn make_line(a: DPoint2, b: DPoint2, w: Double, color: Rgba) -> [Vertex; 6] {
    let v = b - a;
    let l = a.distance(b) * 16.0;
    let t = cgmath::vec2(-v.y / l, v.x / l);
    let t = t / w;

    let a0 = (a + t).cast::<Float>().unwrap();
    let a1 = (a - t).cast::<Float>().unwrap();
    let b0 = (b + t).cast::<Float>().unwrap();
    let b1 = (b - t).cast::<Float>().unwrap();

    let a = Vertex {
        pos: [a0.x, a0.y, 0.0],
        color: color.to_array(),
        normal: [0.0, 0.0, 0.0],
    };
    let b = Vertex {
        pos: [b0.x, b0.y, 0.0],
        color: color.to_array(),
        normal: [0.0, 0.0, 0.0],
    };
    let c = Vertex {
        pos: [a1.x, a1.y, 0.0],
        color: color.to_array(),
        normal: [0.0, 0.0, 0.0],
    };
    let d = Vertex {
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
