use std::fs;

use automancy_system::{
    GameLoadResult,
    game::COULD_NOT_LOAD_ANYTHING,
    game_load_map,
    map::{self, GameMap, LoadMapOption},
    ui_state::{PopupState, Screen, TextField},
};
use automancy_ui::{button, label, row, textbox, window};

use crate::{GameState, event::refresh_maps};

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
                state.ui_state.popup = PopupState::None;
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
                fs::remove_dir_all(
                    GameMap::path(&LoadMapOption::FromSave(map_name.to_string())).unwrap(),
                )
                .unwrap();
                dirty = true;
                state.ui_state.popup = PopupState::None;
                log::info!("Deleted map {map_name}!");
            }

            if button(
                &state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.btn_cancel),
            )
            .clicked
            {
                state.ui_state.popup = PopupState::None
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
            let name = state.ui_state.text_field.get(TextField::MapName);

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
                let name = map::sanitize_name(name.clone());

                state.ui_state.text_field.get(TextField::MapName).clear();
                state.ui_state.popup = PopupState::None;

                match game_load_map(state, name) {
                    GameLoadResult::Loaded => {
                        state.ui_state.switch_screen(Screen::Ingame);
                    }
                    GameLoadResult::LoadedMainMenu => {
                        state.ui_state.switch_screen(Screen::MainMenu);
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
                state.ui_state.popup = PopupState::None
            }
        },
    );
}
