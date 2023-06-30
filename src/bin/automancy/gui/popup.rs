use std::fs;

use egui::{vec2, Align, Align2, Window};

use automancy::game::GameMsg;
use automancy::map::Map;
use automancy::renderer::Renderer;
use automancy_defs::gui::Gui;
use automancy_defs::log;

use crate::event::EventLoopStorage;
use crate::gui::{default_frame, GuiState, PopupState};
use crate::setup::GameSetup;

pub fn invalid_name_popup(setup: &GameSetup, gui: &mut Gui, loop_store: &mut EventLoopStorage) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.invalid_name]
            .as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(250.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
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
            loop_store.popup_state = PopupState::None;
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
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
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
            loop_store.popup_state = PopupState::None;
            log::info!("Deleted map {map_name}!");
        }
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_cancel]
                    .as_str(),
            )
            .clicked()
        {
            loop_store.popup_state = PopupState::None
        }
    });

    if dirty {
        setup.refresh_maps();
    }
}

/// Draws the map creation popup.
pub fn map_create_popup(
    setup: &GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.create_map].as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(250.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
    .show(&gui.context, |ui| {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut loop_store.map_name_input);
        });
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .as_str(),
            )
            .clicked()
        {
            let name = Map::sanitize_name(loop_store.map_name_input.clone());
            setup
                .game
                .send_message(GameMsg::LoadMap(setup.resource_man.clone(), name))
                .unwrap();
            renderer.reset_last_tiles_update();
            loop_store.map_name_input.clear();
            loop_store.popup_state = PopupState::None;
            loop_store.switch_gui_state(GuiState::Ingame);
        }
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_cancel]
                    .as_str(),
            )
            .clicked()
        {
            loop_store.popup_state = PopupState::None
        }
    });
}
