use std::fs;

use egui::{vec2, Align2, Window};
use tokio::runtime::Runtime;

use automancy_defs::gui::Gui;
use automancy_defs::log;

use crate::event::EventLoopStorage;
use crate::game::load_map;
use crate::gui::{PopupState, Screen, TextField};
use crate::map::Map;
use crate::setup::GameSetup;

pub fn invalid_name_popup(setup: &GameSetup, gui: &mut Gui, loop_store: &mut EventLoopStorage) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.invalid_name]
            .as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(250.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(&gui.context, |ui| {
        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.lbl_pick_another_name]
                .as_str(),
        );
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .as_str(),
            )
            .clicked()
        {
            loop_store.gui_state.popup = PopupState::None;
        }
    });
}

pub fn map_delete_popup(
    setup: &mut GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    map_name: &str,
) {
    let mut dirty = false;

    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.delete_map].as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(250.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(&gui.context, |ui| {
        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.lbl_delete_map_confirm]
                .as_str(),
        );
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .as_str(),
            )
            .clicked()
        {
            fs::remove_dir_all(Map::path(map_name)).unwrap();
            dirty = true;
            loop_store.gui_state.popup = PopupState::None;
            log::info!("Deleted map {map_name}!");
        }
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_cancel]
                    .as_str(),
            )
            .clicked()
        {
            loop_store.gui_state.popup = PopupState::None
        }
    });

    if dirty {
        setup.refresh_maps();
    }
}

/// Draws the map creation popup.
pub fn map_create_popup(
    runtime: &Runtime,
    setup: &GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.create_map].as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(250.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(&gui.context, |ui| {
        ui.horizontal(|ui| {
            ui.label("Name:"); //TODO add this to translation
            ui.text_edit_singleline(loop_store.gui_state.text_field.get(TextField::MapName));
        });
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .as_str(),
            )
            .clicked()
        {
            let name = Map::sanitize_name(
                loop_store
                    .gui_state
                    .text_field
                    .get(TextField::MapName)
                    .clone(),
            );

            runtime.block_on(load_map(setup, loop_store, name)).unwrap();

            loop_store
                .gui_state
                .text_field
                .get(TextField::MapName)
                .clear();
            loop_store.gui_state.popup = PopupState::None;
            loop_store.gui_state.switch_screen(Screen::Ingame);
        }
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_cancel]
                    .as_str(),
            )
            .clicked()
        {
            loop_store.gui_state.popup = PopupState::None
        }
    });
}
