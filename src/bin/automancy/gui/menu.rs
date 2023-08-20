use std::error::Error;
use std::fs;

use egui::{
    vec2, Align, Align2, Button, Context, RichText, ScrollArea, TextEdit, TextStyle, Window,
};
use futures::executor::block_on;
use winit::event_loop::ControlFlow;

use automancy::game::GameMsg;
use automancy::map::{Map, MAIN_MENU};
use automancy::VERSION;
use automancy_defs::gui::HyperlinkWidget;
use automancy_defs::log;
use automancy_resources::{format, format_time};

use crate::event::{shutdown_graceful, EventLoopStorage};
use crate::gui::{default_frame, GuiState, PopupState};
use crate::setup::GameSetup;

/// Draws the main menu.
pub fn main_menu(
    setup: &mut GameSetup,
    context: &Context,
    control_flow: &mut ControlFlow,
    loop_store: &mut EventLoopStorage,
) -> Result<bool, Box<dyn Error>> {
    let mut result = Ok(false);

    Window::new("main_menu")
        .resizable(false)
        .title_bar(false)
        .default_width(175.0)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .frame(default_frame())
        .show(context, |ui| {
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
                        result = shutdown_graceful(setup, control_flow);
                    };
                    ui.label(VERSION)
                },
            );
        });

    result
}

/// Draws the pause menu.
pub fn pause_menu(setup: &GameSetup, context: &Context, loop_store: &mut EventLoopStorage) {
    Window::new("Game Paused")
        .resizable(false)
        .collapsible(false)
        .default_width(175.0)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .frame(default_frame())
        .show(context, |ui| {
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
                                    .as_str(),
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
                                    .as_str(),
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
                                    .as_str(),
                            )
                            .heading(),
                        )
                        .clicked()
                    {
                        block_on(setup.game.call(
                            |reply| GameMsg::SaveMap(setup.resource_man.clone(), reply),
                            None,
                        ))
                        .unwrap();
                        setup
                            .game
                            .send_message(GameMsg::LoadMap(
                                setup.resource_man.clone(),
                                MAIN_MENU.to_string(),
                            ))
                            .unwrap();
                        loop_store.switch_gui_state(GuiState::MainMenu)
                    };
                    ui.label(VERSION)
                },
            );
        });
}

/// Draws the map loading menu.
pub fn map_menu(setup: &mut GameSetup, context: &Context, loop_store: &mut EventLoopStorage) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.load_map].as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(600.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .frame(default_frame())
    .show(context, |ui| {
        ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            let mut dirty = false;

            for (map_info, map_name) in &setup.maps {
                ui.group(|ui| {
                    ui.scope(|ui| {
                        ui.style_mut().override_text_style = Some(TextStyle::Heading);
                        ui.set_width(300.0);

                        if Some(map_name) == loop_store.map_name_renaming.as_ref() {
                            if ui
                                .add(
                                    TextEdit::multiline(&mut loop_store.map_name_renaming_input)
                                        .desired_rows(1),
                                )
                                .lost_focus()
                            {
                                loop_store.map_name_renaming_input = loop_store
                                    .map_name_renaming_input
                                    .chars()
                                    .filter(|v| v.is_alphanumeric())
                                    .collect();

                                if fs::rename(
                                    Map::path(map_name),
                                    Map::path(&loop_store.map_name_renaming_input),
                                )
                                .is_ok()
                                {
                                    log::info!(
                                        "Renamed map {map_name} to {}",
                                        loop_store.map_name_renaming_input
                                    );

                                    dirty = true;
                                } else {
                                    loop_store.popup_state = PopupState::InvalidName;
                                }

                                loop_store.map_name_renaming = None;
                                loop_store.map_name_renaming_input = "".to_string();
                            }
                        } else if ui.selectable_label(false, map_name.as_str()).clicked() {
                            loop_store.map_name_renaming = Some(map_name.clone());
                            loop_store.map_name_renaming_input = map_name.clone();
                        }
                    });

                    ui.horizontal(|ui| {
                        if let Some(save_time) = map_info.save_time {
                            ui.label(format_time(
                                save_time,
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.time_fmt]
                                    .as_str(),
                            ));
                        }

                        if ui
                            .button(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_load]
                                    .as_str(),
                            )
                            .clicked()
                        {
                            setup
                                .game
                                .send_message(GameMsg::LoadMap(
                                    setup.resource_man.clone(),
                                    map_name.clone(),
                                ))
                                .unwrap();
                            loop_store.switch_gui_state(GuiState::Ingame);
                        }

                        if ui
                            .button(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_delete]
                                    .as_str(),
                            )
                            .clicked()
                        {
                            loop_store.popup_state =
                                PopupState::MapDeleteConfirmation(map_name.clone());

                            dirty = true;
                        }
                    });
                });
            }

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
                            .as_str(),
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
                            .as_str(),
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

/// Draws the options menu. TODO
pub fn options_menu(setup: &mut GameSetup, context: &Context, loop_store: &mut EventLoopStorage) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.options].as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(175.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .frame(default_frame())
    .show(context, |ui| {
        ui.label("Not yet implemented");
        if ui
            .button(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.btn_confirm]
                    .as_str(),
            )
            .clicked()
        {
            if setup.options.save().is_err() {
                setup.resource_man.error_man.push(
                    (
                        setup.resource_man.registry.err_ids.unwritable_options,
                        vec![],
                    ),
                    &setup.resource_man,
                );
            }
            loop_store.return_gui_state();
        }
    });
}
