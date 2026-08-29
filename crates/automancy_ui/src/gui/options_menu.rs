use automancy_game::persistent::options::{GameOptions, MiscOptions};

use crate::*;

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn options_menu_items(
    ctx: &mut UiContext,
    menu: OptionsMenuState,
    current_options: &mut GameOptions,
    _current_misc_options: &mut MiscOptions,
) {
    match menu {
        OptionsMenuState::Graphics => {
            row_cross_center(|| {
                Text::normal(gui_str!(ctx.game_state, options_graphics_ui_scale)).show();

                let ui_scale_small = gui_str!(ctx.game_state, options_graphics_ui_scale_small);
                let ui_scale_normal = gui_str!(ctx.game_state, options_graphics_ui_scale_normal);
                let ui_scale_large = gui_str!(ctx.game_state, options_graphics_ui_scale_large);

                let new_scale = dropdown_selection(
                    ctx.game_state.options.graphics.ui_scale,
                    StaticList::slice(&[options::UiScale::Small, options::UiScale::Normal, options::UiScale::Large]),
                    move |v| match v {
                        options::UiScale::Small => ui_scale_small,
                        options::UiScale::Normal => ui_scale_normal,
                        options::UiScale::Large => ui_scale_large,
                    },
                    Constraints::loose(Vec2::new(160.0, 180.0)),
                );

                if let Some(new_scale) = new_scale {
                    ctx.game_state.options.graphics.ui_scale = new_scale;
                }
            });

            row_cross_center(|| {
                Text::normal(gui_str!(ctx.game_state, options_graphics_fps)).show();

                if ctx.game_state.options.graphics.fps_limit() == 0 {
                    Text::mono(gui_str!(ctx.game_state, options_graphics_fps_vsync)).show();
                } else if ctx.game_state.options.graphics.fps_limit() == 250 {
                    Text::mono(gui_str!(ctx.game_state, options_graphics_fps_unlimited)).show();
                } else {
                    Text::mono(format!("{: >3}", ctx.game_state.options.graphics.fps_limit())).show();
                }
            });

            if let Some(v) = slider_with_input(ctx.game_state.options.graphics.fps_limit(), 0..=250, None) {
                ctx.game_state.options.graphics.set_fps_limit(v);
            }

            if ctx.render.res.is_vsync() ^ (ctx.game_state.options.graphics.fps_limit() == 0) {
                Text::normal(gui_str!(ctx.game_state, options_graphics_fps_vsync_warning))
                    .color(colors::TEXT_WARNING.yak())
                    .show();
            }

            row_cross_center(|| {
                Text::normal(gui_str!(ctx.game_state, options_graphics_fullscreen)).show();

                checkbox(&mut ctx.game_state.options.graphics.fullscreen);
            });

            row(|| {
                Text::normal(gui_str!(ctx.game_state, options_graphics_antialiasing)).show();

                let aa_none = gui_str!(ctx.game_state, options_graphics_antialiasing_none);
                let aa_fxaa = gui_str!(ctx.game_state, options_graphics_antialiasing_fxaa);

                if let Some(new_antialiasing) = dropdown_selection(
                    ctx.game_state.options.graphics.antialiasing_type,
                    StaticList::slice(&[options::AAType::None, options::AAType::FXAA]),
                    move |v| match v {
                        options::AAType::None => aa_none,
                        options::AAType::FXAA => aa_fxaa,
                    },
                    Constraints::loose(Vec2::new(160.0, 180.0)),
                ) {
                    ctx.game_state.options.graphics.antialiasing_type = new_antialiasing
                }
            });
        },
        OptionsMenuState::Audio => {
            col_cross_center(|| {
                row_cross_center(|| {
                    Text::normal(gui_str!(ctx.game_state, options_audio_sfx_volume)).show();

                    Text::mono(format!("{: >3}%", (ctx.game_state.options.audio.sfx_volume * 100.0) as i32)).show();
                });

                if let Some(v) = slider_with_input(ctx.game_state.options.audio.sfx_volume * 100.0, 0.0..=100.0, Some(1.0)) {
                    ctx.game_state.options.audio.sfx_volume = v / 100.0;
                }
            });

            col_cross_center(|| {
                row_cross_center(|| {
                    Text::normal(gui_str!(ctx.game_state, options_audio_music_volume)).show();

                    Text::mono(format!("{: >3}%", (ctx.game_state.options.audio.music_volume * 100.0) as i32)).show();
                });

                if let Some(v) = slider_with_input(ctx.game_state.options.audio.music_volume * 100.0, 0.0..=100.0, Some(1.0)) {
                    ctx.game_state.options.audio.music_volume = v / 100.0;
                }
            });
        },
        OptionsMenuState::Gui => {
            if !current_options.gui.system_fonts_preferences() {
                row_cross_center(|| {
                    Text::normal(gui_str!(ctx.game_state, options_gui_font)).show();

                    let curr_font: Cow<'static, str> = Cow::Owned(ctx.game_state.options.gui.font().unwrap_or("").to_string());
                    let new_font = dropdown_selection(
                        curr_font,
                        ctx.gui.font_families.clone(),
                        |font| {
                            if font.is_empty() { Cow::Borrowed("???") } else { font.clone() }
                        },
                        Constraints::loose(Vec2::new(280.0, 260.0)),
                    );

                    if let Some(new_font) = new_font {
                        ctx.game_state.options.gui.set_font(Some(new_font.to_string()));
                    }
                });
            }

            row_cross_center(|| {
                Text::normal(gui_str!(ctx.game_state, options_gui_font_system_fonts)).show();

                let mut system_fonts = ctx.game_state.options.gui.system_fonts();
                checkbox(&mut system_fonts);
                ctx.game_state.options.gui.set_system_fonts(system_fonts);
            });

            if ctx.game_state.options.gui.system_fonts() {
                row_cross_center(|| {
                    Text::normal(gui_str!(ctx.game_state, options_gui_font_system_fonts_preferences)).show();

                    let mut system_fonts_preferences = ctx.game_state.options.gui.system_fonts_preferences();
                    checkbox(&mut system_fonts_preferences);
                    ctx.game_state.options.gui.set_system_fonts_preferences(system_fonts_preferences);
                });
            }

            row_cross_center(|| {
                Text::normal(gui_str!(ctx.game_state, options_gui_language)).show();

                Text::normal("TODO: UNIMPLEMENTED").show();
            });
        },
        OptionsMenuState::Controls => {},
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn options_menu(ctx: &mut UiContext, menu: OptionsMenuState) {
    let options_state = use_state(|| None);
    if options_state.borrow().is_none() {
        options_state.set(Some(ctx.game_state.options.clone()));
    }
    let mut options_state = options_state.borrow_mut();
    let current_options = options_state.as_mut().unwrap();

    let misc_options_state = use_state(|| None);
    if misc_options_state.borrow().is_none() {
        misc_options_state.set(Some(ctx.game_state.misc_options.clone()));
    }
    let mut misc_options_state = misc_options_state.borrow_mut();
    let current_misc_options = misc_options_state.as_mut().unwrap();

    let refresh_menu = use_state(|| false);

    // sometimes we'll need to refresh the entire options menu applying the options
    // (e.g. when a font changes, texts need to be recreated to update the font)
    // we can do this silly construct to force this to happen
    // this shouldn't be noticeable anyway :)
    if refresh_menu.get() {
        refresh_menu.set(false);
        return;
    }

    Window::new(gui_str!(ctx.game_state, options_title)).show(Alignment::CENTER, || {
        constrained(Constraints::loose(Vec2::new(ctx.gui.viewport().x * 0.6, f32::INFINITY)), || {
            centered_col(|| {
                Scrollable::horizontal().show(|| {
                    row(|| {
                        if button(gui_str!(ctx.game_state, options_graphics_title)).clicked {
                            ctx.gui.state.screen = Screen::Options(OptionsMenuState::Graphics)
                        }

                        if button(gui_str!(ctx.game_state, options_audio_title)).clicked {
                            ctx.gui.state.screen = Screen::Options(OptionsMenuState::Audio)
                        }

                        if button(gui_str!(ctx.game_state, options_gui_title)).clicked {
                            ctx.gui.state.screen = Screen::Options(OptionsMenuState::Gui)
                        }

                        if button(gui_str!(ctx.game_state, options_controls_title)).clicked {
                            ctx.gui.state.screen = Screen::Options(OptionsMenuState::Controls)
                        }
                    });
                });

                Pad::all(sizing::PADDING_LARGE).show(|| {
                    Scrollable::xy()
                        .child_size(Constraints::loose(Vec2::new(f32::INFINITY, ctx.gui.viewport().y * 0.5)))
                        .min_child_size(Constraints::loose(Vec2::new(320.0, 340.0)))
                        .show(|| {
                            section(|| {
                                list_col_max().cross_axis_alignment(CrossAxisAlignment::Stretch).show(|| {
                                    let title_text_id = match menu {
                                        OptionsMenuState::Graphics => ctx.game_state.resource_man.registry.gui_ids.options_graphics_title,
                                        OptionsMenuState::Audio => ctx.game_state.resource_man.registry.gui_ids.options_audio_title,
                                        OptionsMenuState::Gui => ctx.game_state.resource_man.registry.gui_ids.options_gui_title,
                                        OptionsMenuState::Controls => ctx.game_state.resource_man.registry.gui_ids.options_controls_title,
                                    };

                                    Text::heading(ctx.game_state.resource_man.gui_str(title_text_id)).show();

                                    divider(colors::BACKGROUND_3.yak(), sizing::DIVIER_HEIGHT, sizing::DIVIER_THICKNESS);

                                    col_cross_center(|| {
                                        options_menu_items(ctx, menu, current_options, current_misc_options);
                                    });
                                });
                            });
                        });
                });

                row_spaced_evenly(|| {
                    let finish_res = button(gui_str!(ctx.game_state, options_finish_button));
                    let options_differ = &ctx.game_state.options != current_options || &ctx.game_state.misc_options != current_misc_options;
                    let apply_res = if options_differ {
                        button(gui_str!(ctx.game_state, options_apply_button))
                    } else {
                        inactive_button(gui_str!(ctx.game_state, options_apply_button))
                    };

                    if finish_res.clicked || apply_res.clicked {
                        ctx.game_state.options.mark_out_of_sync();
                        if ctx.game_state.options.save().is_err() {
                            ErrorManager::push_err(
                                &ctx.game_state.resource_man,
                                ctx.game_state.resource_man.registry.err_ids.unwritable_options,
                                [],
                            );
                        }

                        ctx.game_state.misc_options.mark_out_of_sync();
                        if ctx.game_state.misc_options.save().is_err() {
                            ErrorManager::push_err(
                                &ctx.game_state.resource_man,
                                ctx.game_state.resource_man.registry.err_ids.unwritable_options,
                                [],
                            );
                        }

                        *current_options = ctx.game_state.options.clone();
                        *current_misc_options = ctx.game_state.misc_options.clone();
                    }

                    if apply_res.clicked {
                        refresh_menu.set(true);
                    }

                    if finish_res.clicked {
                        ctx.gui.state.return_screen();
                    }
                });
            });
        });
    });
}
