use std::{fs, mem};

use automancy_data::{colors::BACKGROUND_3, math::vec2};
use automancy_resources::{error::push_err, format::FormatContext, format_time};
use automancy_system::{
    GameLoadResult,
    game::{COULD_NOT_LOAD_ANYTHING, GameSystemMessage},
    game_load_map, game_load_map_inner,
    map::{GameMap, LoadMapOption},
    options::UiScale,
    ui_state::{OptionsMenuState, PopupState, Screen, SubState, TextField},
};
use automancy_ui::{
    DIVIER_HEIGHT, DIVIER_THICKNESS, PADDING_LARGE, PADDING_MEDIUM, PADDING_SMALL, button,
    center_col, center_row, checkbox, col, group, heading, label, pad_x, row,
    scroll_horizontal_bar_alignment, scroll_vertical, selection_box, slider, stretch_col, textbox,
    window,
};
use interpolator::Formattable;
use winit::event_loop::ActiveEventLoop;
use yakui::{Constraints, Vec2, constrained, divider, image, spacer, widgets::Pad};

use crate::{
    GameState, VERSION,
    event::{refresh_maps, shutdown_graceful},
};

/// Draws the main menu.
pub fn main_menu(state: &mut GameState, event_loop: &ActiveEventLoop) -> anyhow::Result<bool> {
    let mut result = Ok(false);

    window("Main Menu".to_string(), || {
        image(state.logo.unwrap(), Vec2::new(128.0, 128.0));

        if button(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.btn_play),
        )
        .clicked
        {
            refresh_maps(state);
            state.ui_state.switch_screen(Screen::MapLoad)
        };

        if button(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.btn_options),
        )
        .clicked
        {
            state.ui_state.switch_screen(Screen::Options)
        };

        if button(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.btn_fedi),
        )
        .clicked
        {
            open::that_detached("https://gamedev.lgbt/@automancy").unwrap();
        }

        if button(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.btn_source),
        )
        .clicked
        {
            open::that_detached("https://github.com/automancy/automancy").unwrap();
        }

        if button(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.btn_exit),
        )
        .clicked
        {
            result = state.tokio.block_on(shutdown_graceful(
                &state.game,
                &mut state.game_handle,
                event_loop,
            ));
        };

        label(VERSION);
    });

    result
}

/// Draws the pause menu.
pub fn pause_menu(state: &mut GameState) {
    window("Game Paused".to_string(), || {
        if button(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.btn_unpause),
        )
        .clicked
        {
            state.ui_state.switch_screen(Screen::Ingame)
        };

        if button(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.btn_options),
        )
        .clicked
        {
            state.ui_state.switch_screen(Screen::Options)
        };

        if button(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.btn_exit),
        )
        .clicked
        {
            state
                .tokio
                .block_on(state.game.call(GameSystemMessage::SaveMap, None))
                .unwrap()
                .unwrap();

            assert!(
                game_load_map_inner(state, LoadMapOption::MainMenu) != GameLoadResult::Failed,
                "{}",
                COULD_NOT_LOAD_ANYTHING
            );

            state.ui_state.switch_screen(Screen::MainMenu)
        };

        label(VERSION);
    });
}

/// Draws the map loading menu.
pub fn map_menu(state: &mut GameState) {
    window(
        state
            .resource_man
            .gui_str(state.resource_man.registry.gui_ids.load_map)
            .to_string(),
        || {
            scroll_vertical(
                Vec2::ZERO,
                Vec2::new(state.ui_viewport().x * 0.7, 260.0),
                || {
                    stretch_col(|| {
                        let mut dirty = false;

                        {
                            let infos = std::mem::take(&mut state.loop_store.map_infos_cache);
                            for ((_, save_time), map_name) in &infos {
                                group(|| {
                                    row(|| {
                                        Pad::vertical(PADDING_SMALL).show(|| {
                                            if Some(map_name)
                                                == state.ui_state.renaming_map.as_ref()
                                            {
                                                let renaming = state
                                                    .ui_state
                                                    .text_field
                                                    .get(TextField::MapRenaming);

                                                let res = textbox(renaming, None);
                                                if res.lost_focus || res.activated {
                                                    state.ui_state.renaming_map = None;

                                                    let new_name = std::mem::take(renaming)
                                                        .chars()
                                                        .filter(|v| v.is_alphanumeric())
                                                        .collect::<String>();

                                                    if fs::rename(
                                                        GameMap::path(&LoadMapOption::FromSave(
                                                            map_name.clone(),
                                                        ))
                                                        .unwrap(),
                                                        GameMap::path(&LoadMapOption::FromSave(
                                                            new_name.clone(),
                                                        ))
                                                        .unwrap(),
                                                    )
                                                    .is_ok()
                                                    {
                                                        log::info!(
                                                            "Renamed map {map_name} to {new_name}"
                                                        );

                                                        dirty = true;
                                                    } else {
                                                        state.ui_state.popup =
                                                            PopupState::InvalidName;
                                                    }
                                                }
                                            } else if button(map_name).clicked {
                                                state
                                                    .ui_state
                                                    .text_field
                                                    .get(TextField::MapRenaming)
                                                    .clone_from(map_name);
                                                state.ui_state.renaming_map =
                                                    Some(map_name.clone());
                                            }
                                        });
                                    });

                                    row(|| {
                                        if let Some(save_time) = save_time {
                                            label(&format_time(
                                                *save_time,
                                                &state.resource_man.gui_str(
                                                    state.resource_man.registry.gui_ids.time_fmt,
                                                ),
                                            ));
                                        }

                                        spacer(1);

                                        row(|| {
                                            if button(&state.resource_man.gui_str(
                                                state.resource_man.registry.gui_ids.btn_load,
                                            ))
                                            .clicked
                                            {
                                                match game_load_map(state, map_name.clone()) {
                                                    GameLoadResult::Loaded => {
                                                        state
                                                            .ui_state
                                                            .switch_screen(Screen::Ingame);
                                                    }
                                                    GameLoadResult::LoadedMainMenu => {
                                                        state
                                                            .ui_state
                                                            .switch_screen(Screen::MainMenu);
                                                    }
                                                    GameLoadResult::Failed => {
                                                        panic!("{}", COULD_NOT_LOAD_ANYTHING)
                                                    }
                                                }
                                            }
                                            if button(&state.resource_man.gui_str(
                                                state.resource_man.registry.gui_ids.btn_delete,
                                            ))
                                            .clicked
                                            {
                                                state.ui_state.popup =
                                                    PopupState::MapDeleteConfirmation(
                                                        map_name.clone(),
                                                    );

                                                dirty = true;
                                            }
                                        });
                                    });
                                });
                            }
                            state.loop_store.map_infos_cache = infos;
                        }

                        if dirty {
                            refresh_maps(state);
                        }
                    });
                },
            );

            label(&state.resource_man.gui_fmt(
                state.resource_man.registry.gui_ids.lbl_maps_loaded,
                [(
                    "maps_number",
                    Formattable::integer(&state.loop_store.map_infos_cache.len()),
                )],
            ));

            row(|| {
                if button(
                    &state
                        .resource_man
                        .gui_str(state.resource_man.registry.gui_ids.btn_new_map),
                )
                .clicked
                {
                    state.ui_state.popup = PopupState::MapCreate
                }

                if button(
                    &state
                        .resource_man
                        .gui_str(state.resource_man.registry.gui_ids.btn_cancel),
                )
                .clicked
                {
                    state.ui_state.switch_screen(Screen::MainMenu)
                }
            });
        },
    );
}

pub fn options_menu_item(state: &mut GameState, menu: OptionsMenuState) {
    match menu {
        OptionsMenuState::Graphics => {
            center_row(|| {
                // TODO translate these
                label(
                    &state.resource_man.gui_str(
                        state
                            .resource_man
                            .registry
                            .gui_ids
                            .options_graphics_ui_scale,
                    ),
                );

                let new_scale = selection_box(
                    [UiScale::Small, UiScale::Normal, UiScale::Large],
                    state.options.graphics.ui_scale,
                    &|v| match v {
                        UiScale::Small => state.resource_man.gui_str(
                            state
                                .resource_man
                                .registry
                                .gui_ids
                                .options_graphics_ui_scale_small,
                        ),
                        UiScale::Normal => state.resource_man.gui_str(
                            state
                                .resource_man
                                .registry
                                .gui_ids
                                .options_graphics_ui_scale_normal,
                        ),
                        UiScale::Large => state.resource_man.gui_str(
                            state
                                .resource_man
                                .registry
                                .gui_ids
                                .options_graphics_ui_scale_large,
                        ),
                    },
                );

                if new_scale != state.options.graphics.ui_scale {
                    state.options.graphics.ui_scale = new_scale;

                    state.gui.as_mut().unwrap().yak.set_scale_factor(
                        (state.renderer.as_ref().unwrap().gpu.window.scale_factor()
                            * state.options.graphics.ui_scale.to_f64())
                            as f32,
                    );
                }
            });

            center_col(|| {
                label(&format!(
                    "Max FPS: {: >3}",
                    if state.options.graphics.fps_limit == 0 {
                        "Vsync".to_string()
                    } else if state.options.graphics.fps_limit == 250 {
                        "Unlimited".to_string()
                    } else {
                        state.options.graphics.fps_limit.to_string()
                    }
                ));

                slider(
                    &mut state.options.graphics.fps_limit,
                    0..=250,
                    None,
                    |v| v.parse().ok(),
                    |v| format!("{: >3}", v),
                );
            });

            center_col(|| {
                label("Fullscreen: ");

                checkbox(&mut state.options.graphics.fullscreen);
            });

            /*
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
            }); */
        }
        OptionsMenuState::Audio => {
            center_col(|| {
                label(&format!(
                    "SFX Volume: {: >3}%",
                    (state.options.audio.sfx_volume * 100.0) as i32
                ));

                slider(
                    &mut state.options.audio.sfx_volume,
                    0.0..=1.0,
                    Some(0.01),
                    |v| v.parse::<f64>().ok().map(|v| v / 100.0),
                    |v| format!("{: >3}", (v * 100.0) as i32),
                );
            });

            center_col(|| {
                label(&format!(
                    "Music Volume: {: >3}%",
                    (state.options.audio.music_volume * 100.0) as i32
                ));

                slider(
                    &mut state.options.audio.music_volume,
                    0.0..=1.0,
                    Some(0.01),
                    |v| v.parse::<f64>().ok().map(|v| v / 100.0),
                    |v| format!("{: >3}", (v * 100.0) as i32),
                );
            });
        }
        OptionsMenuState::Gui => {
            center_col(|| {
                label("Font:");

                let new_font = selection_box(
                    state.resource_man.fonts.keys().cloned().map(Some),
                    state.options.gui.get_font(&state.resource_man),
                    &|font| font.clone().unwrap_or_default(),
                );

                state.options.gui.set_font(&state.resource_man, new_font);
            });

            center_col(|| {
                label("Language:");

                label("TODO: UNIMPLEMENTED");
            });
        }
        OptionsMenuState::Controls => {}
    }
}

/// Draws the options menu.
pub fn options_menu(state: &mut GameState) {
    window(
        state
            .resource_man
            .gui_str(state.resource_man.registry.gui_ids.options)
            .to_string(),
        || {
            center_col(|| {
                scroll_horizontal_bar_alignment(Vec2::ZERO, Vec2::INFINITY, None, || {
                    row(|| {
                        if button(
                            &state
                                .resource_man
                                .gui_str(state.resource_man.registry.gui_ids.options_graphics),
                        )
                        .clicked
                        {
                            state.ui_state.substate = SubState::Options(OptionsMenuState::Graphics)
                        }

                        if button(
                            &state
                                .resource_man
                                .gui_str(state.resource_man.registry.gui_ids.options_audio),
                        )
                        .clicked
                        {
                            state.ui_state.substate = SubState::Options(OptionsMenuState::Audio)
                        }

                        if button(
                            &state
                                .resource_man
                                .gui_str(state.resource_man.registry.gui_ids.options_gui),
                        )
                        .clicked
                        {
                            state.ui_state.substate = SubState::Options(OptionsMenuState::Gui)
                        }

                        if button(
                            &state
                                .resource_man
                                .gui_str(state.resource_man.registry.gui_ids.options_controls),
                        )
                        .clicked
                        {
                            state.ui_state.substate = SubState::Options(OptionsMenuState::Controls)
                        }
                    });
                });

                Pad::all(PADDING_LARGE).show(|| {
                    if let SubState::Options(menu) = state.ui_state.substate {
                        scroll_vertical(Vec2::ZERO, Vec2::new(f32::INFINITY, 260.0), || {
                            group(|| {
                                col(|| {
                                    heading(&state.resource_man.gui_str(match menu {
                                        OptionsMenuState::Graphics => {
                                            state.resource_man.registry.gui_ids.options_graphics
                                        }
                                        OptionsMenuState::Audio => {
                                            state.resource_man.registry.gui_ids.options_audio
                                        }
                                        OptionsMenuState::Gui => {
                                            state.resource_man.registry.gui_ids.options_gui
                                        }
                                        OptionsMenuState::Controls => {
                                            state.resource_man.registry.gui_ids.options_controls
                                        }
                                    }));

                                    divider(BACKGROUND_3, DIVIER_HEIGHT, DIVIER_THICKNESS);

                                    constrained(
                                        Constraints {
                                            min: Vec2::new(280.0, 300.0),
                                            max: Vec2::INFINITY,
                                        },
                                        || {
                                            pad_x(0.0, PADDING_MEDIUM).show(|| {
                                                center_col(|| {
                                                    options_menu_item(state, menu);
                                                });
                                            });
                                        },
                                    );
                                });
                            });
                        });
                    }
                });

                row(|| {
                    if button(
                        &state
                            .resource_man
                            .gui_str(state.resource_man.registry.gui_ids.btn_confirm),
                    )
                    .clicked
                    {
                        if state.options.save().is_err() {
                            push_err(
                                state.resource_man.registry.err_ids.unwritable_options,
                                &FormatContext::from([].into_iter()),
                                &state.resource_man,
                            );
                        }

                        if state.misc_options.save().is_err() {
                            push_err(
                                state.resource_man.registry.err_ids.unwritable_options,
                                &FormatContext::from([].into_iter()),
                                &state.resource_man,
                            );
                        }

                        state.ui_state.return_screen();
                    }
                });
            });
        },
    );
}
