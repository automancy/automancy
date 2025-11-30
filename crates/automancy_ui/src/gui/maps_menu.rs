use crate::*;

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn map_name_invalid_popup(ctx: &mut UiContext) {
    Window::new(gui_str!(ctx.game_state, map_invalid_name_title)).show(Alignment::CENTER, || {
        col_cross_center(|| {
            Text::normal(gui_str!(ctx.game_state, map_invalid_name_label)).show();

            let confirm_res = button(gui_str!(ctx.game_state, map_invalid_name_accept_button));

            if confirm_res.clicked {
                ctx.gui.state.popup = PopupState::None;
            }
        });
    });
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn map_delete_popup(ctx: &mut UiContext, map_name: &str) {
    let mut dirty = false;

    let confirm_counter = use_state(|| 0);

    Window::new(gui_str!(ctx.game_state, map_delete_title)).show(Alignment::CENTER, || {
        col_cross_center(|| {
            Text::normal(gui_str!(ctx.game_state, map_delete_label)).show();

            let confirm_res = button(gui_str!(ctx.game_state, map_delete_confirm_button));
            if confirm_res.clicked && confirm_counter.get() == 0 {
                confirm_counter.set(1);
            }

            if confirm_counter.get() >= 1 {
                Text::normal(gui_str!(ctx.game_state, map_delete_confirm_warning))
                    .color(colors::TEXT_WARNING.yak())
                    .padding(Pad::all(sizing::PADDING_XLARGE))
                    .show();

                let confirm_res = button(gui_str!(ctx.game_state, map_delete_confirm_again_button));
                if confirm_res.clicked {
                    confirm_counter.set(0);

                    fs::remove_dir_all(GameMap::path(GameMapId::SaveFile(SaveFileName::new(map_name))).unwrap()).unwrap();
                    dirty = true;

                    ctx.gui.state.popup = PopupState::None;
                    log::info!("Deleted map '{map_name}'!");
                }
            }

            let cancel_res = button(gui_str!(ctx.game_state, map_delete_cancel_button));

            if cancel_res.clicked {
                ctx.gui.state.popup = PopupState::None
            }
        });
    });

    if dirty {
        ctx.game_state.refresh_maps_cache(ctx.game_data);
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn map_create_popup(ctx: &mut UiContext) {
    Window::new(gui_str!(ctx.game_state, map_create_title)).show(Alignment::CENTER, || {
        col_cross_center(|| {
            Pad::none().bottom(sizing::PADDING_MEDIUM).show(|| {
                row_cross_center(|| {
                    Text::normal(gui_str!(ctx.game_state, map_create_name_input_label)).show();

                    textbox(
                        ctx.gui.state.text_field.get(TextField::MapName),
                        TextStyle::normal(),
                        Some(gui_str!(ctx.game_state, map_create_name_input_placeholder).into()),
                        None,
                    );
                });
            });

            let confirm_res = button(gui_str!(ctx.game_state, map_create_confirm_button));
            let cancel_res = button(gui_str!(ctx.game_state, map_create_cancel_button));

            if confirm_res.clicked {
                let name = ctx.gui.state.text_field.take(TextField::MapName);
                ctx.gui.state.popup = PopupState::None;

                match ctx
                    .game_state
                    .load_map(ctx.game_data, GameMapId::SaveFile(SaveFileName::new(&name)))
                {
                    AutomancyGameLoadResult::Loaded => {
                        ctx.gui.state.switch_screen(Screen::Ingame);
                    },
                    AutomancyGameLoadResult::LoadedMainMenu => {
                        ctx.gui.state.switch_screen(Screen::MainMenu);
                    },
                    AutomancyGameLoadResult::Failed => {
                        panic!();
                    },
                }
            }

            if cancel_res.clicked {
                ctx.gui.state.popup = PopupState::None
            }
        });
    });
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
#[inline]
#[must_use]
fn map_name_widget(ctx: &mut UiContext, map_name: &str) -> bool {
    let mut dirty = false;

    let renaming_text = use_state(|| "".to_string());
    let is_renaming = use_state(|| false);
    let rename_button_clicked = use_state(|| false);

    row(|| {
        Pad::vertical(sizing::PADDING_SMALL).show(|| {
            if !is_renaming.get() {
                let rename_button_res = button(map_name.to_string());

                if rename_button_res.clicked {
                    renaming_text.set(map_name.to_string());
                    is_renaming.set(true);
                    rename_button_clicked.set(true);
                }
            } else {
                let rename_textbox_res = textbox(&mut renaming_text.borrow_mut(), TextStyle::normal(), None, None);
                if rename_button_clicked.get() {
                    ctx.gui.focus_next_frame = FocusState::Set(rename_textbox_res.id);
                    rename_button_clicked.set(false);
                }

                if (rename_textbox_res.lost_focus || rename_textbox_res.activated) && map_name != *renaming_text.borrow() {
                    let new_name = std::mem::take(&mut *renaming_text.borrow_mut());
                    is_renaming.set(false);

                    let old_path = GameMap::path(GameMapId::SaveFile(SaveFileName::new(map_name))).unwrap();
                    let new_name = new_name.chars().filter(|v| v.is_alphanumeric()).collect::<String>();
                    log::info!("Renaming map '{map_name}' to '{new_name}'.");

                    if let Some(new_path) = GameMap::path(GameMapId::SaveFile(SaveFileName::new(&new_name)))
                        && fs::rename(old_path, new_path).is_ok()
                    {
                        log::info!("Rename successful!");

                        dirty = true;
                    } else {
                        log::info!("Rename failed!");

                        ctx.gui.state.popup = PopupState::InvalidName;
                    }
                }
            }
        });
    });

    dirty
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn map_menu(ctx: &mut UiContext) {
    let mut dirty = false;

    let map_infos = std::mem::take(&mut ctx.game_data.map_infos);

    Window::new(gui_str!(ctx.game_state, map_list_title)).show(Alignment::CENTER, || {
        col_cross_center(|| {
            Scrollable::xy()
                .child_size(Constraints::loose(Vec2::new(
                    ctx.gui.viewport().x * 0.7,
                    ctx.gui.viewport().y * 0.6,
                )))
                .min_child_size(Constraints::loose(Vec2::new(400.0, f32::INFINITY)))
                .show(|| {
                    col(|| {
                        for (map_name, (_, save_time)) in map_infos.iter() {
                            section(|| {
                                col(|| {
                                    dirty = map_name_widget(ctx, map_name) || dirty;

                                    row_max(|| {
                                        if let Some(save_time) = save_time {
                                            Text::normal(
                                                DateTime::<Local>::from(*save_time)
                                                    .format(gui_str!(ctx.game_state, time_fmt))
                                                    .to_string(),
                                            )
                                            .show();
                                        }

                                        spacer(1);

                                        row(|| {
                                            let load_button_res = button(gui_str!(ctx.game_state, map_list_load_button));
                                            if load_button_res.clicked {
                                                match ctx
                                                    .game_state
                                                    .load_map(ctx.game_data, GameMapId::SaveFile(SaveFileName::new(map_name)))
                                                {
                                                    AutomancyGameLoadResult::Loaded => {
                                                        ctx.gui.state.switch_screen(Screen::Ingame);
                                                    },
                                                    AutomancyGameLoadResult::LoadedMainMenu => {
                                                        ctx.gui.state.switch_screen(Screen::MainMenu);
                                                    },
                                                    AutomancyGameLoadResult::Failed => {
                                                        panic!();
                                                    },
                                                }
                                            }

                                            let delete_button_res = button(gui_str!(ctx.game_state, map_list_delete_button));
                                            if delete_button_res.clicked {
                                                ctx.gui.state.popup = PopupState::MapDeleteConfirmation(map_name.clone());

                                                dirty = true;
                                            }
                                        });
                                    });
                                });
                            });
                        }
                    });
                });

            Text::normal(gui_str!(
                ctx.game_state,
                map_list_total_label,
                [("maps_number", Formattable::integer(&map_infos.len()))].into_iter()
            ))
            .show();

            row(|| {
                let new_map_button_res = button(gui_str!(ctx.game_state, map_list_new_map_button));

                if new_map_button_res.clicked {
                    ctx.gui.state.popup = PopupState::MapCreate
                }

                let cancel_res = button(gui_str!(ctx.game_state, map_list_exit_button));

                if cancel_res.clicked {
                    ctx.gui.state.switch_screen(Screen::MainMenu)
                }
            });
        });
    });

    ctx.game_data.map_infos = map_infos;

    if dirty {
        ctx.game_state.refresh_maps_cache(ctx.game_data);
    }
}
