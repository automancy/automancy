use std::borrow::Cow;
use std::fs;

use egui::load::Bytes;
use egui::{
    vec2, Align, Align2, Button, Checkbox, ComboBox, Image, ImageSource, RichText, ScrollArea,
    Slider, TextEdit, TextStyle, Window,
};
use winit::event_loop::EventLoopWindowTarget;

use automancy_defs::gui::HyperlinkWidget;
use automancy_defs::log;
use automancy_resources::{format, format_time};

use crate::event::{refresh_maps, shutdown_graceful};
use crate::game::{load_map, GameSystemMessage};
use crate::gui::{OptionsMenuState, PopupState, Screen, SubState, TextField};
use crate::map::{Map, MAIN_MENU};
use crate::options::AAType;
use crate::{GameState, LOGO, LOGO_PATH, VERSION};

/// Draws the main menu.
pub fn main_menu(
    state: &mut GameState,
    target: &EventLoopWindowTarget<()>,
) -> anyhow::Result<bool> {
    let mut result = Ok(false);

    Window::new("main_menu")
        .resizable(false)
        .title_bar(false)
        .max_width(200.0)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(&state.gui.context.clone(), |ui| {
            ui.with_layout(
                ui.layout()
                    .with_cross_align(Align::Center)
                    .with_main_align(Align::Center),
                |ui| {
                    ui.add(
                        Image::new(ImageSource::Bytes {
                            uri: Cow::Borrowed(LOGO_PATH),
                            bytes: Bytes::Static(LOGO),
                        })
                        .max_size(vec2(128.0, 128.0)),
                    );

                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    state.resource_man.translates.gui
                                        [&state.resource_man.registry.gui_ids.btn_play]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        refresh_maps(state);
                        state.gui_state.switch_screen(Screen::MapLoad)
                    };

                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    state.resource_man.translates.gui
                                        [&state.resource_man.registry.gui_ids.btn_options]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        state.gui_state.switch_screen(Screen::Options)
                    };

                    ui.add(HyperlinkWidget::new(
                        Button::new(
                            RichText::new(
                                state.resource_man.translates.gui
                                    [&state.resource_man.registry.gui_ids.btn_fedi]
                                    .as_str(),
                            )
                            .heading(),
                        )
                        .min_size(vec2(100.0, 28.0)),
                        "https://gamedev.lgbt/@automancy",
                    ));

                    ui.add(HyperlinkWidget::new(
                        Button::new(
                            RichText::new(
                                state.resource_man.translates.gui
                                    [&state.resource_man.registry.gui_ids.btn_source]
                                    .as_str(),
                            )
                            .heading(),
                        )
                        .min_size(vec2(100.0, 28.0)),
                        "https://github.com/automancy/automancy",
                    ));

                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    state.resource_man.translates.gui
                                        [&state.resource_man.registry.gui_ids.btn_exit]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        result = state.tokio.block_on(shutdown_graceful(
                            &state.game,
                            &mut state.game_handle,
                            target,
                        ));
                    };

                    ui.label(VERSION)
                },
            );
        });

    result
}

/// Draws the pause menu.
pub fn pause_menu(state: &mut GameState) {
    Window::new("Game Paused")
        .resizable(false)
        .collapsible(false)
        .default_width(175.0)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(&state.gui.context.clone(), |ui| {
            ui.with_layout(
                ui.layout()
                    .with_cross_align(Align::Center)
                    .with_main_align(Align::Center),
                |ui| {
                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    state.resource_man.translates.gui
                                        [&state.resource_man.registry.gui_ids.btn_unpause]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        state.gui_state.switch_screen(Screen::Ingame)
                    };
                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    state.resource_man.translates.gui
                                        [&state.resource_man.registry.gui_ids.btn_options]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        state.gui_state.switch_screen(Screen::Options)
                    };
                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    state.resource_man.translates.gui
                                        [&state.resource_man.registry.gui_ids.btn_exit]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
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
                    ui.label(VERSION)
                },
            );
        });
}

/// Draws the map loading menu.
pub fn map_menu(state: &mut GameState) {
    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.load_map].as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(600.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(&state.gui.context.clone(), |ui| {
        ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            let mut dirty = false;

            for ((_, save_time), map_name) in state.loop_store.map_infos_cache.clone() {
                ui.group(|ui| {
                    ui.scope(|ui| {
                        ui.style_mut().override_text_style = Some(TextStyle::Heading);
                        ui.set_width(300.0);

                        if map_name == state.gui_state.renaming_map {
                            if ui
                                .add(
                                    TextEdit::multiline(
                                        state.gui_state.text_field.get(TextField::MapRenaming),
                                    )
                                    .desired_rows(1),
                                )
                                .lost_focus()
                            {
                                let s = state
                                    .gui_state
                                    .text_field
                                    .take(TextField::MapRenaming)
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
                        } else if ui.selectable_label(false, map_name.as_str()).clicked() {
                            state.gui_state.renaming_map = map_name.clone();
                        }
                    });

                    if let Some(save_time) = save_time {
                        ui.label(format_time(
                            save_time,
                            state.resource_man.translates.gui
                                [&state.resource_man.registry.gui_ids.time_fmt]
                                .as_str(),
                        ));
                    }

                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                state.resource_man.translates.gui
                                    [&state.resource_man.registry.gui_ids.btn_load]
                                    .as_str(),
                            )
                            .clicked()
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
                        if ui
                            .button(
                                state.resource_man.translates.gui
                                    [&state.resource_man.registry.gui_ids.btn_delete]
                                    .as_str(),
                            )
                            .clicked()
                        {
                            state.gui_state.popup =
                                PopupState::MapDeleteConfirmation(map_name.clone());

                            dirty = true;
                        }
                    });
                });
            }

            if dirty {
                refresh_maps(state);
            }
        });
        ui.label(format(
            state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.lbl_maps_loaded]
                .as_str(),
            &[state.loop_store.map_infos_cache.len().to_string().as_str()],
        ));
        ui.horizontal(|ui| {
            if ui
                .button(
                    RichText::new(
                        state.resource_man.translates.gui
                            [&state.resource_man.registry.gui_ids.btn_new_map]
                            .as_str(),
                    )
                    .heading(),
                )
                .clicked()
            {
                state.gui_state.popup = PopupState::MapCreate
            }
            if ui
                .button(
                    RichText::new(
                        state.resource_man.translates.gui
                            [&state.resource_man.registry.gui_ids.btn_cancel]
                            .as_str(),
                    )
                    .heading(),
                )
                .clicked()
            {
                state.gui_state.switch_screen(Screen::MainMenu)
            }
        });
    });
}

/// Draws the options menu.
pub fn options_menu(state: &mut GameState) {
    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.options].as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(175.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(&state.gui.context.clone(), |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                if ui
                    .add(Button::new("Graphics").min_size(vec2(80.0, 24.0)))
                    .clicked()
                {
                    state.gui_state.substate = SubState::Options(OptionsMenuState::Graphics)
                }
                if ui
                    .add(Button::new("Audio").min_size(vec2(80.0, 24.0)))
                    .clicked()
                {
                    state.gui_state.substate = SubState::Options(OptionsMenuState::Audio)
                }
                if ui
                    .add(Button::new("GUI").min_size(vec2(80.0, 24.0)))
                    .clicked()
                {
                    state.gui_state.substate = SubState::Options(OptionsMenuState::Gui)
                }
                if ui
                    .add(Button::new("Controls").min_size(vec2(80.0, 24.0)))
                    .clicked()
                {
                    state.gui_state.substate = SubState::Options(OptionsMenuState::Controls)
                }
            });

            ScrollArea::vertical().show(ui, |ui| {
                if let SubState::Options(menu) = state.gui_state.substate {
                    match menu {
                        OptionsMenuState::Graphics => {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Graphics").text_style(TextStyle::Heading));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Max FPS: "));
                                    ui.add(
                                        Slider::new(
                                            &mut state.options.graphics.fps_limit,
                                            0.0..=250.0,
                                        )
                                        .step_by(5.0)
                                        .custom_formatter(
                                            |n, _| {
                                                if n == 0.0 {
                                                    "Vsync".to_string()
                                                } else if n == 250.0 {
                                                    "Unlimited".to_string()
                                                } else {
                                                    format!("{}", n)
                                                }
                                            },
                                        ),
                                    )
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Fullscreen: "));
                                    ui.add(Checkbox::new(
                                        &mut state.options.graphics.fullscreen,
                                        "",
                                    ));
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Scale: "));
                                    ui.add(
                                        Slider::new(&mut state.options.graphics.scale, 0.5..=4.0)
                                            .step_by(0.5),
                                    )
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Antialiasing: "));
                                    ComboBox::from_label("")
                                        .selected_text(format!(
                                            "{:?}",
                                            state.options.graphics.anti_aliasing //TODO inconsistent, use a to_string?
                                        ))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut state.options.graphics.anti_aliasing,
                                                AAType::None,
                                                "None",
                                            );
                                            ui.selectable_value(
                                                &mut state.options.graphics.anti_aliasing,
                                                AAType::FXAA,
                                                "FXAA",
                                            );
                                            ui.selectable_value(
                                                &mut state.options.graphics.anti_aliasing,
                                                AAType::TAA,
                                                "TAA",
                                            );
                                        });
                                })
                            });
                        }
                        OptionsMenuState::Audio => {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Audio").text_style(TextStyle::Heading));
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("SFX Volume: "));
                                    ui.add(
                                        Slider::new(&mut state.options.audio.sfx_volume, 0.0..=1.0)
                                            .custom_formatter(|n, _| {
                                                if n == 0.0 {
                                                    return "Muted".to_string();
                                                };
                                                format!("{}%", (n * 100.0) as usize)
                                            }),
                                    )
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Music Volume: "));
                                    ui.add(
                                        Slider::new(
                                            &mut state.options.audio.music_volume,
                                            0.0..=1.0,
                                        )
                                        .custom_formatter(
                                            |n, _| {
                                                if n == 0.0 {
                                                    return "Muted".to_string();
                                                };
                                                format!("{}%", (n * 100.0) as usize)
                                            },
                                        ),
                                    )
                                });
                            });
                        }
                        OptionsMenuState::Gui => {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("GUI").text_style(TextStyle::Heading));
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Font Scale: "));
                                    ui.add(
                                        Slider::new(&mut state.options.gui.scale, 0.5..=4.0)
                                            .step_by(0.25),
                                    )
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Font:"));
                                    let current_font = state.options.gui.font.clone();

                                    ComboBox::from_label("")
                                        .width(175.0)
                                        .selected_text(
                                            &state.resource_man.fonts[&current_font].name,
                                        )
                                        .show_ui(ui, |ui| {
                                            for (key, font) in &state.resource_man.fonts {
                                                ui.selectable_value(
                                                    &mut state.options.gui.font,
                                                    key.to_string(),
                                                    font.name.clone(),
                                                )
                                                .on_hover_text(key.to_string());
                                            }
                                        });
                                });
                            });
                        }
                        OptionsMenuState::Controls => {
                            ui.label(RichText::new("Controls").text_style(TextStyle::Heading));
                        }
                    }
                }
            });
        });

        if ui
            .add(
                Button::new(RichText::new(
                    state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.btn_confirm]
                        .as_str(),
                ))
                .min_size(vec2(80.0, 24.0)),
            )
            .clicked()
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
}
