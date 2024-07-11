use std::fs;

use crate::game::{load_map, COULD_NOT_LOAD_ANYTHING};
use crate::gui::{PopupState, Screen, TextField};
use crate::map::Map;
use crate::GameState;
use crate::{event::refresh_maps, game::GameLoadResult};

use super::{button, label, row, textbox, window};

pub fn invalid_name_popup(state: &mut GameState) {
    window(
        state
            .resource_man
            .gui_str(state.resource_man.registry.gui_ids.invalid_name)
            .to_string(),
        || {
            label(
                &state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.lbl_pick_another_name),
            );

            if button(
                &state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.btn_confirm),
            )
            .clicked
            {
                state.gui_state.popup = PopupState::None;
            }
        },
    );
}

pub fn map_delete_popup(state: &mut GameState, map_name: &str) {
    let mut dirty = false;

    window(
        state
            .resource_man
            .gui_str(state.resource_man.registry.gui_ids.delete_map)
            .to_string(),
        || {
            label(
                &state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.lbl_delete_map_confirm),
            );

            if button(
                &state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.btn_confirm),
            )
            .clicked
            {
                fs::remove_dir_all(Map::path(map_name)).unwrap();
                dirty = true;
                state.gui_state.popup = PopupState::None;
                log::info!("Deleted map {map_name}!");
            }

            if button(
                &state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.btn_cancel),
            )
            .clicked
            {
                state.gui_state.popup = PopupState::None
            }
        },
    );

    if dirty {
        refresh_maps(state);
    }
}

/// Draws the map creation popup.
pub fn map_create_popup(state: &mut GameState) {
    window(
        state
            .resource_man
            .gui_str(state.resource_man.registry.gui_ids.create_map)
            .to_string(),
        || {
            let name = state.gui_state.text_field.get(TextField::MapName);

            row(|| {
                label("Name:"); //TODO add this to translation

                textbox(name, None, Some("Name your world here..."));
            });

            if button(
                &state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.btn_confirm),
            )
            .clicked
            {
                let name = Map::sanitize_name(name.clone());

                state.gui_state.text_field.get(TextField::MapName).clear();
                state.gui_state.popup = PopupState::None;

                match load_map(state, name, false) {
                    GameLoadResult::Loaded => {
                        state.gui_state.switch_screen(Screen::Ingame);
                    }
                    GameLoadResult::LoadedMainMenu => {
                        state.gui_state.switch_screen(Screen::MainMenu);
                    }
                    GameLoadResult::Failed => {
                        panic!("{}", COULD_NOT_LOAD_ANYTHING)
                    }
                }
            }

            if button(
                &state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.btn_cancel),
            )
            .clicked
            {
                state.gui_state.popup = PopupState::None
            }
        },
    );
}
