use std::borrow::Cow;
use std::fs;

use egui::load::Bytes;
use egui::{
    vec2, Align, Align2, Button, Checkbox, ComboBox, Context, Image, ImageSource, RichText,
    ScrollArea, Slider, TextEdit, TextStyle, Window,
};
use tokio::runtime::Runtime;
use winit::event_loop::EventLoopWindowTarget;

use automancy_defs::flexstr::ToSharedStr;
use automancy_defs::gui::HyperlinkWidget;
use automancy_defs::log;
use automancy_resources::{format, format_time};

use crate::event::{shutdown_graceful, EventLoopStorage};
use crate::game::{load_map, GameMsg};
use crate::gui::{OptionsMenuState, PopupState, Screen, SubState, TextField};
use crate::map::{Map, MAIN_MENU};
use crate::options::AAType;
use crate::setup::GameSetup;
use crate::{LOGO, LOGO_PATH, VERSION};

/// Draws the main menu.
pub fn main_menu(
    runtime: &Runtime,
    setup: &mut GameSetup,
    context: &Context,
    target: &EventLoopWindowTarget<()>,
    loop_store: &mut EventLoopStorage,
) -> anyhow::Result<bool> {
    let mut result = Ok(false);

    Window::new("main_menu")
        .resizable(false)
        .title_bar(false)
        .max_width(200.0)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(context, |ui| {
            ui.with_layout(
                ui.layout()
                    .with_cross_align(Align::Center)
                    .with_main_align(Align::Center),
                |ui| {
                    ui.add(
                        Image::new(ImageSource::Bytes {
                            uri: Cow::Owned(LOGO_PATH.to_string()), // TODO this is a bit expensive?
                            bytes: Bytes::Static(LOGO),
                        })
                        .max_size(vec2(128.0, 128.0)),
                    );
                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    setup.resource_man.translates.gui
                                        [&setup.resource_man.registry.gui_ids.btn_play]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        setup.refresh_maps();
                        loop_store.gui_state.switch_screen(Screen::MapLoad)
                    };
                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    setup.resource_man.translates.gui
                                        [&setup.resource_man.registry.gui_ids.btn_options]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        loop_store.gui_state.switch_screen(Screen::Options)
                    };

                    ui.add(HyperlinkWidget::new(
                        Button::new(
                            RichText::new(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_fedi]
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
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_source]
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
                                    setup.resource_man.translates.gui
                                        [&setup.resource_man.registry.gui_ids.btn_exit]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        result = runtime.block_on(shutdown_graceful(setup, target));
                    };
                    ui.label(VERSION)
                },
            );
        });

    result
}

/// Draws the pause menu.
pub fn pause_menu(
    runtime: &Runtime,
    setup: &GameSetup,
    context: &Context,
    loop_store: &mut EventLoopStorage,
) {
    Window::new("Game Paused")
        .resizable(false)
        .collapsible(false)
        .default_width(175.0)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(context, |ui| {
            ui.with_layout(
                ui.layout()
                    .with_cross_align(Align::Center)
                    .with_main_align(Align::Center),
                |ui| {
                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    setup.resource_man.translates.gui
                                        [&setup.resource_man.registry.gui_ids.btn_unpause]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        loop_store.gui_state.switch_screen(Screen::Ingame)
                    };
                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    setup.resource_man.translates.gui
                                        [&setup.resource_man.registry.gui_ids.btn_options]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        loop_store.gui_state.switch_screen(Screen::Options)
                    };
                    if ui
                        .add(
                            Button::new(
                                RichText::new(
                                    setup.resource_man.translates.gui
                                        [&setup.resource_man.registry.gui_ids.btn_exit]
                                        .as_str(),
                                )
                                .heading(),
                            )
                            .min_size(vec2(100.0, 28.0)),
                        )
                        .clicked()
                    {
                        runtime
                            .block_on(setup.game.call(GameMsg::SaveMap, None))
                            .unwrap()
                            .unwrap();

                        runtime
                            .block_on(load_map(setup, loop_store, MAIN_MENU.to_string()))
                            .unwrap();

                        loop_store.gui_state.switch_screen(Screen::MainMenu)
                    };
                    ui.label(VERSION)
                },
            );
        });
}

/// Draws the map loading menu.
pub fn map_menu(
    runtime: &Runtime,
    setup: &mut GameSetup,
    context: &Context,
    loop_store: &mut EventLoopStorage,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.load_map].as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(600.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(context, |ui| {
        ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            let mut dirty = false;

            for ((_, save_time), map_name) in &setup.maps {
                ui.group(|ui| {
                    ui.scope(|ui| {
                        ui.style_mut().override_text_style = Some(TextStyle::Heading);
                        ui.set_width(300.0);

                        if map_name == loop_store.gui_state.text_field.get(TextField::MapRenaming) {
                            if ui
                                .add(
                                    TextEdit::multiline(
                                        loop_store.gui_state.text_field.get(TextField::MapRenaming),
                                    )
                                    .desired_rows(1),
                                )
                                .lost_focus()
                            {
                                *loop_store.gui_state.text_field.get(TextField::MapRenaming) =
                                    loop_store
                                        .gui_state
                                        .text_field
                                        .get(TextField::MapRenaming)
                                        .chars()
                                        .filter(|v| v.is_alphanumeric())
                                        .collect();

                                if fs::rename(
                                    Map::path(map_name),
                                    Map::path(
                                        loop_store.gui_state.text_field.get(TextField::MapRenaming),
                                    ),
                                )
                                .is_ok()
                                {
                                    log::info!(
                                        "Renamed map {map_name} to {}",
                                        loop_store.gui_state.text_field.get(TextField::MapRenaming)
                                    );

                                    dirty = true;
                                } else {
                                    loop_store.gui_state.popup = PopupState::InvalidName;
                                }

                                *loop_store.gui_state.text_field.get(TextField::MapRenaming) =
                                    Default::default();
                            }
                        } else if ui.selectable_label(false, map_name.as_str()).clicked() {
                            *loop_store.gui_state.text_field.get(TextField::MapRenaming) =
                                map_name.clone();
                        }
                    });

                    ui.horizontal(|ui| {
                        if let Some(save_time) = save_time {
                            ui.label(format_time(
                                *save_time,
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
                            runtime
                                .block_on(load_map(setup, loop_store, map_name.clone()))
                                .unwrap();

                            loop_store.gui_state.switch_screen(Screen::Ingame);
                        }

                        if ui
                            .button(
                                setup.resource_man.translates.gui
                                    [&setup.resource_man.registry.gui_ids.btn_delete]
                                    .as_str(),
                            )
                            .clicked()
                        {
                            loop_store.gui_state.popup =
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
                loop_store.gui_state.popup = PopupState::MapCreate
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
                loop_store.gui_state.switch_screen(Screen::MainMenu)
            }
        });
    });
}

/// Draws the options menu.
pub fn options_menu(setup: &mut GameSetup, context: &Context, loop_store: &mut EventLoopStorage) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.options].as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .default_width(175.0)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .show(context, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                if ui
                    .add(Button::new("Graphics").min_size(vec2(80.0, 24.0)))
                    .clicked()
                {
                    loop_store.gui_state.substate = SubState::Options(OptionsMenuState::Graphics)
                }
                if ui
                    .add(Button::new("Audio").min_size(vec2(80.0, 24.0)))
                    .clicked()
                {
                    loop_store.gui_state.substate = SubState::Options(OptionsMenuState::Audio)
                }
                if ui
                    .add(Button::new("GUI").min_size(vec2(80.0, 24.0)))
                    .clicked()
                {
                    loop_store.gui_state.substate = SubState::Options(OptionsMenuState::Gui)
                }
                if ui
                    .add(Button::new("Controls").min_size(vec2(80.0, 24.0)))
                    .clicked()
                {
                    loop_store.gui_state.substate = SubState::Options(OptionsMenuState::Controls)
                }
            });
            ScrollArea::vertical().show(ui, |ui| {
                if let SubState::Options(menu) = loop_store.gui_state.substate {
                    match menu {
                        OptionsMenuState::Graphics => {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Graphics").text_style(TextStyle::Heading));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Max FPS: "));
                                    ui.add(
                                        Slider::new(
                                            &mut setup.options.graphics.fps_limit,
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
                                        &mut setup.options.graphics.fullscreen,
                                        "",
                                    ));
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Scale: "));
                                    ui.add(
                                        Slider::new(&mut setup.options.graphics.scale, 0.5..=4.0)
                                            .step_by(0.5),
                                    )
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Antialiasing: "));
                                    ComboBox::from_label("")
                                        .selected_text(format!(
                                            "{:?}",
                                            setup.options.graphics.anti_aliasing
                                        ))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut setup.options.graphics.anti_aliasing,
                                                AAType::None,
                                                "None",
                                            );
                                            ui.selectable_value(
                                                &mut setup.options.graphics.anti_aliasing,
                                                AAType::FXAA,
                                                "FXAA",
                                            );
                                            ui.selectable_value(
                                                &mut setup.options.graphics.anti_aliasing,
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
                                        Slider::new(&mut setup.options.audio.sfx_volume, 0.0..=1.0)
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
                                            &mut setup.options.audio.music_volume,
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
                                        Slider::new(&mut setup.options.gui.scale, 0.5..=4.0)
                                            .step_by(0.25),
                                    )
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Font:"));
                                    ComboBox::from_label("")
                                        .width(175.0)
                                        .selected_text(
                                            &setup.resource_man.fonts
                                                [&setup.options.gui.font.to_shared_str()]
                                                .name,
                                        )
                                        .show_ui(ui, |ui| {
                                            for (key, font) in &setup.resource_man.fonts {
                                                ui.selectable_value(
                                                    &mut setup.options.gui.font,
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
                    setup.resource_man.translates.gui
                        [&setup.resource_man.registry.gui_ids.btn_confirm]
                        .as_str(),
                ))
                .min_size(vec2(80.0, 24.0)),
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
            loop_store.gui_state.return_screen();
        }
    });
}
