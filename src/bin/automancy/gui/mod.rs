use egui::epaint::Shadow;
use egui::{Frame, Rounding, ScrollArea, Ui};
use fuse_rust::Fuse;

use automancy_defs::colors;
use automancy_defs::id::Id;
use automancy_defs::math::Float;
use automancy_resources::ResourceManager;

const MARGIN: Float = 8.0;
const ITEM_ICON_SIZE: Float = 24.0;

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

    ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
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
