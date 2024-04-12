use std::{fs, mem};

use winit::event_loop::EventLoopWindowTarget;

use automancy_defs::{glam::vec2, log};
use automancy_resources::{format, format_time};
use yakui::{
    checkbox, column, image, row, slider, textbox,
    widgets::{List, Slider},
};

use crate::event::{refresh_maps, shutdown_graceful};
use crate::game::{load_map, GameSystemMessage};
use crate::gui::{OptionsMenuState, PopupState, Screen, SubState, TextField};
use crate::map::{Map, MAIN_MENU};
use crate::options::AAType;
use crate::{GameState, VERSION};

use super::components::{
    button::button,
    container::group,
    layout::centered_column,
    scrollable::{scroll_vertical, Scrollable},
    select::selection_box,
    text::{heading, label},
    window::window,
};

/// Draws the main menu.
pub fn main_menu(
    state: &mut GameState,
    target: &EventLoopWindowTarget<()>,
) -> anyhow::Result<bool> {
    let mut result = Ok(false);

    window("Main Menu".to_string(), || {
        centered_column(|| {
            image(state.logo, vec2(128.0, 128.0));

            if button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_play]
                    .as_str(),
            )
            .clicked
            {
                refresh_maps(state);
                state.gui_state.switch_screen(Screen::MapLoad)
            };

            if button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_options]
                    .as_str(),
            )
            .clicked
            {
                state.gui_state.switch_screen(Screen::Options)
            };

            if button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_fedi]
                    .as_str(),
            )
            .clicked
            {
                open::that("https://gamedev.lgbt/@automancy").unwrap();
            }

            if button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_source]
                    .as_str(),
            )
            .clicked
            {
                open::that("https://github.com/automancy/automancy").unwrap();
            }

            if button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_exit]
                    .as_str(),
            )
            .clicked
            {
                result = state.tokio.block_on(shutdown_graceful(
                    &state.game,
                    &mut state.game_handle,
                    target,
                ));
            };

            label(VERSION);
        });
    });

    result
}

/// Draws the pause menu.
pub fn pause_menu(state: &mut GameState) {
    window("Game Paused".to_string(), || {
        centered_column(|| {
            if button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_unpause]
                    .as_str(),
            )
            .clicked
            {
                state.gui_state.switch_screen(Screen::Ingame)
            };

            if button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_options]
                    .as_str(),
            )
            .clicked
            {
                state.gui_state.switch_screen(Screen::Options)
            };

            if button(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_exit]
                    .as_str(),
            )
            .clicked
            {
                state
                    .tokio
                    .block_on(state.game.call(GameSystemMessage::SaveMap, None))
                    .unwrap()
                    .unwrap();

                state
                    .tokio
                    .block_on(load_map(
                        &state.game,
                        &mut state.loop_store,
                        MAIN_MENU.to_string(),
                    ))
                    .unwrap();

                state.gui_state.switch_screen(Screen::MainMenu)
            };

            label(VERSION);
        });
    });
}

/// Draws the map loading menu.
pub fn map_menu(state: &mut GameState) {
    window(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.load_map]
            .to_string(),
        || {
            centered_column(|| {
                scroll_vertical(400.0, || {
                    let mut dirty = false;

                    for ((_, save_time), map_name) in state.loop_store.map_infos_cache.clone() {
                        group(|| {
                            if map_name == state.gui_state.renaming_map {
                                let renaming =
                                    state.gui_state.text_field.get(TextField::MapRenaming);
                                let mut text = textbox(renaming.clone());
                                if let Some(new_name) = text.text.take() {
                                    *renaming = new_name
                                }

                                if text.lost_focus {
                                    let s = mem::take(renaming)
                                        .chars()
                                        .filter(|v| v.is_alphanumeric())
                                        .collect::<String>();

                                    if fs::rename(Map::path(&map_name), Map::path(&s)).is_ok() {
                                        log::info!("Renamed map {map_name} to {}", s);

                                        dirty = true;
                                    } else {
                                        state.gui_state.popup = PopupState::InvalidName;
                                    }
                                }
                            } else if button(map_name.as_str()).clicked {
                                state.gui_state.renaming_map = map_name.clone();
                                *state.gui_state.text_field.get(TextField::MapRenaming) =
                                    map_name.clone();
                            }

                            List::row().show(|| {
                                if let Some(save_time) = save_time {
                                    label(&format_time(
                                        save_time,
                                        &state.resource_man.translates.gui
                                            [&state.resource_man.registry.gui_ids.time_fmt],
                                    ));
                                }

                                row(|| {
                                    if button(
                                        state.resource_man.translates.gui
                                            [&state.resource_man.registry.gui_ids.btn_load]
                                            .as_str(),
                                    )
                                    .clicked
                                    {
                                        state
                                            .tokio
                                            .block_on(load_map(
                                                &state.game,
                                                &mut state.loop_store,
                                                map_name.clone(),
                                            ))
                                            .unwrap();

                                        state.gui_state.switch_screen(Screen::Ingame);
                                    }
                                    if button(
                                        state.resource_man.translates.gui
                                            [&state.resource_man.registry.gui_ids.btn_delete]
                                            .as_str(),
                                    )
                                    .clicked
                                    {
                                        state.gui_state.popup =
                                            PopupState::MapDeleteConfirmation(map_name.clone());

                                        dirty = true;
                                    }
                                });
                            });
                        });
                    }

                    if dirty {
                        refresh_maps(state);
                    }
                });

                label(&format(
                    &state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.lbl_maps_loaded],
                    &[&state.loop_store.map_infos_cache.len().to_string()],
                ));

                row(|| {
                    if button(
                        state.resource_man.translates.gui
                            [&state.resource_man.registry.gui_ids.btn_new_map]
                            .as_str(),
                    )
                    .clicked
                    {
                        state.gui_state.popup = PopupState::MapCreate
                    }

                    if button(
                        state.resource_man.translates.gui
                            [&state.resource_man.registry.gui_ids.btn_cancel]
                            .as_str(),
                    )
                    .clicked
                    {
                        state.gui_state.switch_screen(Screen::MainMenu)
                    }
                });
            });
        },
    );
}

/// Draws the options menu.
pub fn options_menu(state: &mut GameState) {
    window(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.options].to_string(),
        || {
            centered_column(|| {
                row(|| {
                    column(|| {
                        // TODO translate these
                        if button("Graphics").clicked {
                            state.gui_state.substate = SubState::Options(OptionsMenuState::Graphics)
                        }

                        if button("Audio").clicked {
                            state.gui_state.substate = SubState::Options(OptionsMenuState::Audio)
                        }

                        if button("GUI").clicked {
                            state.gui_state.substate = SubState::Options(OptionsMenuState::Gui)
                        }

                        if button("Controls").clicked {
                            state.gui_state.substate = SubState::Options(OptionsMenuState::Controls)
                        }
                    });

                    scroll_vertical(200.0, || {
                        if let SubState::Options(menu) = state.gui_state.substate {
                            match menu {
                                OptionsMenuState::Graphics => {
                                    column(|| {
                                        heading("Graphics");

                                        column(|| {
                                            label("Max FPS: ");
                                            if let Some(v) =
                                                slider(state.options.graphics.fps_limit, 0.0, 250.0)
                                                    .value
                                            {
                                                state.options.graphics.fps_limit = v;
                                            } /* TODO custom fps widget
                                              if n == 0.0 {
                                                  "Vsync".to_string()
                                              } else if n == 250.0 {
                                                  "Unlimited".to_string()
                                              } else {
                                                  format!("{}", n)
                                              } */
                                        });

                                        row(|| {
                                            label("Fullscreen: ");

                                            state.options.graphics.fullscreen =
                                                checkbox(state.options.graphics.fullscreen).checked;
                                        });

                                        row(|| {
                                            label("Scale: ");

                                            let mut slider =
                                                Slider::new(state.options.graphics.scale, 0.5, 4.0);
                                            slider.step = Some(0.5);

                                            if let Some(v) = slider.show().value {
                                                state.options.graphics.scale = v;
                                            }
                                        });

                                        row(|| {
                                            label("Antialiasing: ");

                                            state.options.graphics.anti_aliasing = selection_box(
                                                [AAType::None, AAType::FXAA, AAType::TAA],
                                                state.options.graphics.anti_aliasing,
                                                &|v| {
                                                    format!(
                                                        "{:?}",
                                                        v //TODO inconsistent, use a to_string?
                                                    )
                                                },
                                            );
                                        });
                                    });
                                }
                                OptionsMenuState::Audio => {
                                    column(|| {
                                        heading("Audio");

                                        row(|| {
                                            label("SFX Volume: ");
                                            if let Some(v) =
                                                slider(state.options.audio.sfx_volume, 0.0, 1.0)
                                                    .value
                                            {
                                                state.options.audio.sfx_volume = v;
                                            }
                                        });

                                        row(|| {
                                            label("Music Volume: ");
                                            if let Some(v) =
                                                slider(state.options.audio.music_volume, 0.0, 1.0)
                                                    .value
                                            {
                                                state.options.audio.music_volume = v;
                                            }
                                        });
                                    });
                                }
                                OptionsMenuState::Gui => {
                                    column(|| {
                                        heading("GUI");

                                        row(|| {
                                            label("Font:");

                                            state.options.gui.font = selection_box(
                                                state.gui.font_names.keys().cloned(),
                                                state.options.gui.font.clone(),
                                                &|font| state.gui.font_names[font].to_string(),
                                            );
                                        });
                                    });
                                }
                                OptionsMenuState::Controls => {
                                    heading("Controls");
                                }
                            }
                        }
                    });
                });

                if button(
                    &state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.btn_confirm],
                )
                .clicked
                {
                    if state.options.save().is_err() {
                        state.resource_man.error_man.push(
                            (
                                state.resource_man.registry.err_ids.unwritable_options,
                                vec![],
                            ),
                            &state.resource_man,
                        );
                    }
                    state.gui_state.return_screen();
                }
            });
        },
    );
}
