use std::fs;

use automancy_defs::log;
use yakui::row;

use crate::event::refresh_maps;
use crate::game::load_map;
use crate::gui::{PopupState, Screen, TextField};
use crate::map::Map;
use crate::GameState;

use super::components::{
    button::button, layout::centered_column, text::label, textbox::textbox, window::window,
};

pub fn invalid_name_popup(state: &mut GameState) {
    window(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.invalid_name]
            .to_string(),
        || {
            centered_column(|| {
                label(
                    state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.lbl_pick_another_name]
                        .as_str(),
                );

                if button(
                    state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.btn_confirm]
                        .as_str(),
                )
                .clicked
                {
                    state.gui_state.popup = PopupState::None;
                }
            });
        },
    );
}

pub fn map_delete_popup(state: &mut GameState, map_name: &str) {
    let mut dirty = false;

    window(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.delete_map]
            .to_string(),
        || {
            centered_column(|| {
                label(
                    state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.lbl_delete_map_confirm]
                        .as_str(),
                );

                if button(
                    state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.btn_confirm]
                        .as_str(),
                )
                .clicked
                {
                    fs::remove_dir_all(Map::path(map_name)).unwrap();
                    dirty = true;
                    state.gui_state.popup = PopupState::None;
                    log::info!("Deleted map {map_name}!");
                }

                if button(
                    state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.btn_cancel]
                        .as_str(),
                )
                .clicked
                {
                    state.gui_state.popup = PopupState::None
                }
            });
        },
    );

    if dirty {
        refresh_maps(state);
    }
}

/// Draws the map creation popup.
pub fn map_create_popup(state: &mut GameState) {
    window(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.create_map]
            .to_string(),
        || {
            centered_column(|| {
                let name = state.gui_state.text_field.get(TextField::MapName);

                row(|| {
                    label("Name:"); //TODO add this to translation
                    if let Some(new_name) = textbox(name, "").text.take() {
                        *name = new_name;
                    }
                });

                if button(
                    &state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.btn_confirm],
                )
                .clicked
                {
                    let name = Map::sanitize_name(name.clone());

                    state
                        .tokio
                        .block_on(load_map(&state.game, &mut state.loop_store, name))
                        .unwrap();

                    state.gui_state.text_field.get(TextField::MapName).clear();
                    state.gui_state.popup = PopupState::None;
                    state.gui_state.switch_screen(Screen::Ingame);
                }

                if button(
                    state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.btn_cancel]
                        .as_str(),
                )
                .clicked
                {
                    state.gui_state.popup = PopupState::None
                }
            });
        },
    );
}
