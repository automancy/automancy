use std::fs;

use futures::executor::block_on;

use automancy::game::GameMsg;
use automancy::map::{Map, MapInfo};
use automancy::renderer::Renderer;
use automancy::VERSION;
use automancy_defs::egui::{vec2, Align, Align2, Button, RichText, ScrollArea, Window};
use automancy_defs::egui_winit_vulkano::Gui;
use automancy_defs::gui::HyperlinkWidget;
use automancy_defs::log;
use automancy_defs::winit::event_loop::ControlFlow;
use automancy_resources::{format, unix_to_formatted_time};

use crate::event::{shutdown_graceful, EventLoopStorage};
use crate::gui::{default_frame, GuiState, PopupState};
use crate::setup::GameSetup;

/// Draws the main menu.
pub fn main_menu(
    setup: &mut GameSetup,
    gui: &mut Gui,
    control_flow: &mut ControlFlow,
    loop_store: &mut EventLoopStorage,
) {
    Window::new("main_menu".to_string())
        .resizable(false)
        .default_width(175.0)
        .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
        .frame(default_frame().inner_margin(10.0))
        .movable(false)
        .title_bar(false)
        .show(&gui.context(), |ui| {
            ui.with_layout(
                ui.layout()
                    .with_cross_align(Align::Center)
                    .with_main_align(Align::Center),
                |ui| {
                    ui.label(RichText::new("automancy").size(30.0));
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_play]
                                    .as_str(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        setup.refresh_maps();
                        loop_store.switch_gui_state(GuiState::MapLoad)
                    };
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_options]
                                    .as_str(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        loop_store.switch_gui_state(GuiState::Options)
                    };

                    ui.add(HyperlinkWidget::new(
                        Button::new(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_fedi]
                                    .as_str(),
                            )
                            .heading(),
                        ),
                        "https://gamedev.lgbt/@automancy",
                    ));
                    ui.add(HyperlinkWidget::new(
                        Button::new(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_source]
                                    .as_str(),
                            )
                            .heading(),
                        ),
                        "https://github.com/sorcerers-class/automancy",
                    ));
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_exit]
                                    .as_str(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        shutdown_graceful(setup, control_flow)
                            .expect("Failed to shutdown gracefully!");
                    };
                    ui.label(VERSION)
                },
            );
        });
}

/// Draws the pause menu.
pub fn pause_menu(
    setup: &mut GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
) {
    Window::new("Game Paused".to_string())
        .resizable(false)
        .default_width(175.0)
        .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
        .frame(default_frame().inner_margin(10.0))
        .movable(false)
        .show(&gui.context(), |ui| {
            ui.with_layout(
                ui.layout()
                    .with_cross_align(Align::Center)
                    .with_main_align(Align::Center),
                |ui| {
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_unpause]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        loop_store.switch_gui_state(GuiState::Ingame)
                    };
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_options]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        loop_store.switch_gui_state(GuiState::Options)
                    };
                    if ui
                        .button(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_exit]
                                    .to_string(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        // block the current thread so the map can save to completion
                        block_on(setup.game.call(
                            |reply| GameMsg::SaveMap(setup.resource_man.clone(), reply),
                            None,
                        ))
                        .unwrap();
                        setup
                            .game
                            .send_message(GameMsg::LoadMap(
                                setup.resource_man.clone(),
                                ".mainmenu".to_string(),
                            ))
                            .unwrap();
                        renderer.reset_last_tiles_update();
                        loop_store.switch_gui_state(GuiState::MainMenu)
                    };
                    ui.label(VERSION)
                },
            );
        });
}

/// Draws the map loading menu.
pub fn map_load_menu(
    setup: &mut GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.load_map]
            .to_string(),
    )
    .resizable(false)
    .default_width(250.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
    .show(&gui.context(), |ui| {
        ScrollArea::vertical().max_height(225.0).show(ui, |ui| {
            let dirty = false;
            setup.maps.iter().for_each(|map| {
                let resource_man = setup.resource_man.clone();
                let time = unix_to_formatted_time(
                    map.save_time,
                    resource_man.translates.gui[&resource_man.registry.gui_ids.time_fmt].as_str(),
                );
                ui.group(|ui| {
                    ui.label(RichText::new(&map.map_name).heading());
                    ui.horizontal(|ui| {
                        ui.label(time);
                        if ui
                            .button(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_load]
                                    .to_string(),
                            )
                            .clicked()
                        {
                            setup
                                .game
                                .send_message(GameMsg::LoadMap(resource_man, map.map_name.clone()))
                                .unwrap();
                            renderer.reset_last_tiles_update();
                            loop_store.switch_gui_state(GuiState::Ingame);
                        }
                        if ui
                            .button(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_delete]
                                    .to_string(),
                            )
                            .clicked()
                        {
                            loop_store.popup_state = PopupState::MapDeleteConfirmation(map.clone());
                        }
                    });
                });
            });
            if dirty {
                setup.refresh_maps();
            }
        });
        ui.label(format(
            setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.lbl_maps_loaded]
                .as_str(),
            &[setup.maps.len().to_string().as_str()],
        ));
        ui.horizontal(|ui| {
            if ui
                .button(
                    RichText::new(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.btn_new_map]
                            .to_string(),
                    )
                    .heading(),
                )
                .clicked()
            {
                loop_store.popup_state = PopupState::MapCreate
            }
            if ui
                .button(
                    RichText::new(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.btn_cancel]
                            .to_string(),
                    )
                    .heading(),
                )
                .clicked()
            {
                loop_store.switch_gui_state(GuiState::MainMenu)
            }
        });
    });
}

pub fn map_delete_confirmation(
    setup: &mut GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    map: MapInfo,
) {
    let mut dirty = false;
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.delete_map]
            .to_string(),
    )
    .resizable(false)
    .default_width(250.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
    .show(&gui.context(), |ui| {
        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.lbl_delete_map_confirm]
                .to_string(),
        );
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .to_string(),
            )
            .clicked()
        {
            fs::remove_file(format!("map/{}.run", map.map_name)).unwrap();
            dirty = true;
            loop_store.popup_state = PopupState::None;
            log::info!("Deleted map {}!", map.map_name);
        }
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_cancel]
                    .to_string(),
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
pub fn map_create_menu(
    setup: &mut GameSetup,
    gui: &mut Gui,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.create_map]
            .to_string(),
    )
    .resizable(false)
    .default_width(250.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
    .show(&gui.context(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut loop_store.filter);
        });
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .to_string(),
            )
            .clicked()
        {
            let name = Map::sanitize_name(loop_store.filter.clone()); //TODO WHAT THE FUCK IS IT DOING
            setup
                .game
                .send_message(GameMsg::LoadMap(setup.resource_man.clone(), name))
                .unwrap();
            renderer.reset_last_tiles_update();
            loop_store.filter.clear();
            loop_store.popup_state = PopupState::None;
            loop_store.switch_gui_state(GuiState::Ingame);
        }
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_cancel]
                    .to_string(),
            )
            .clicked()
        {
            loop_store.popup_state = PopupState::None
        }
    });
}

/// Draws the options menu. TODO
pub fn options_menu(setup: &mut GameSetup, gui: &mut Gui, loop_store: &mut EventLoopStorage) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.options].as_str(),
    )
    .resizable(false)
    .default_width(175.0)
    .anchor(Align2([Align::Center, Align::Center]), vec2(0.0, 0.0))
    .frame(default_frame().inner_margin(10.0))
    .show(&gui.context(), |ui| {
        ui.label("Not yet implemented");
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .as_str(),
            )
            .clicked()
        {
            loop_store.return_gui_state();
        }
    });
}
