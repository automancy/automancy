use std::fs;

use egui::{vec2, Align2, Window};

use automancy_defs::log;

use crate::event::refresh_maps;
use crate::game::load_map;
use crate::gui::{PopupState, Screen, TextField};
use crate::map::Map;
use crate::GameState;

pub fn invalid_name_popup(state: &mut GameState) {
    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.invalid_name]
            .as_str(),
    )
    .id("invalid_map_name_popup".into())
    .resizable(false)
    .collapsible(false)
    .default_width(250.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(&state.gui.context.clone(), |ui| {
        ui.label(
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.lbl_pick_another_name]
                .as_str(),
        );
        if ui
            .button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_confirm]
                    .as_str(),
            )
            .clicked()
        {
            state.gui_state.popup = PopupState::None;
        }
    });
}

pub fn map_delete_popup(state: &mut GameState, map_name: &str) {
    let mut dirty = false;

    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.delete_map].as_str(),
    )
    .id("map_delete_popup".into())
    .resizable(false)
    .collapsible(false)
    .default_width(250.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(&state.gui.context.clone(), |ui| {
        ui.label(
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.lbl_delete_map_confirm]
                .as_str(),
        );
        if ui
            .button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_confirm]
                    .as_str(),
            )
            .clicked()
        {
            fs::remove_dir_all(Map::path(map_name)).unwrap();
            dirty = true;
            state.gui_state.popup = PopupState::None;
            log::info!("Deleted map {map_name}!");
        }
        if ui
            .button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_cancel]
                    .as_str(),
            )
            .clicked()
        {
            state.gui_state.popup = PopupState::None
        }
    });

    if dirty {
        refresh_maps(state);
    }
}

/// Draws the map creation popup.
pub fn map_create_popup(state: &mut GameState) {
    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.create_map].as_str(),
    )
    .id("map_create_popup".into())
    .resizable(false)
    .collapsible(false)
    .default_width(250.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(&state.gui.context.clone(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Name:"); //TODO add this to translation
            ui.text_edit_singleline(state.gui_state.text_field.get(TextField::MapName));
        });
        if ui
            .button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_confirm]
                    .as_str(),
            )
            .clicked()
        {
            let name =
                Map::sanitize_name(state.gui_state.text_field.get(TextField::MapName).clone());

            state
                .tokio
                .block_on(load_map(&state.game, &mut state.loop_store, name))
                .unwrap();

            state.gui_state.text_field.get(TextField::MapName).clear();
            state.gui_state.popup = PopupState::None;
            state.gui_state.switch_screen(Screen::Ingame);
        }
        if ui
            .button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_cancel]
                    .as_str(),
            )
            .clicked()
        {
            state.gui_state.popup = PopupState::None
        }
    });
}
