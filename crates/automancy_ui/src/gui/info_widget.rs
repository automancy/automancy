use crate::*;

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn input_hint_names(ctx: &mut UiContext) {
    for hint in &ctx.gui.state.input_hints {
        let name = hint
            .last()
            .and_then(|ty| ctx.game_state.input_handler.key_map.values().find(|v| v.ty == *ty))
            .and_then(|v| v.name);

        Text::normal(ctx.game_state.resource_man.try_key_name(name)).show();
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn input_hint_keys(ctx: &mut UiContext) {
    for hint in &ctx.gui.state.input_hints {
        let hint_text = hint
            .iter()
            .flat_map(|ty| {
                if let Some((key, _)) = ctx.game_state.input_handler.key_map.iter().find(|(_, v)| v.ty == *ty) {
                    if let Key::Character(c) = key {
                        Some(c.to_uppercase())
                    } else if let Key::Named(n) = key {
                        match n {
                            NamedKey::Alt => Some("Alt".to_string()),
                            NamedKey::Control => Some("Ctrl".to_string()),
                            NamedKey::Shift => Some("Shift".to_string()),
                            NamedKey::Delete => Some("Del".to_string()),
                            NamedKey::Backspace => Some("Backspace".to_string()),
                            NamedKey::Enter => Some("Enter".to_string()),
                            NamedKey::Escape => Some("Esc".to_string()),
                            NamedKey::Tab => Some("Tab".to_string()),
                            NamedKey::F1 => Some("F1".to_string()),
                            NamedKey::F2 => Some("F2".to_string()),
                            NamedKey::F3 => Some("F3".to_string()),
                            NamedKey::F4 => Some("F4".to_string()),
                            NamedKey::F5 => Some("F5".to_string()),
                            NamedKey::F6 => Some("F6".to_string()),
                            NamedKey::F7 => Some("F7".to_string()),
                            NamedKey::F8 => Some("F8".to_string()),
                            NamedKey::F9 => Some("F9".to_string()),
                            NamedKey::F10 => Some("F10".to_string()),
                            NamedKey::F11 => Some("F11".to_string()),
                            NamedKey::F12 => Some("F12".to_string()),
                            NamedKey::ArrowLeft => Some("Left".to_string()),
                            NamedKey::ArrowUp => Some("Up".to_string()),
                            NamedKey::ArrowDown => Some("Down".to_string()),
                            NamedKey::ArrowRight => Some("Right".to_string()),
                            _ => Some("<?>".to_string()),
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" + ");

        Text::normal(hint_text).color(colors::TEXT_INACTIVE.yak()).show();
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn rest_of_the_info(ctx: &mut UiContext) {
    Scrollable::horizontal().show(|| {
        section(|| {
            row(|| {
                col(|| {
                    input_hint_names(ctx);
                });

                col_cross_end(|| {
                    input_hint_keys(ctx);
                });
            });
        });
    });
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn info_widget(ctx: &mut UiContext) {
    Window::new(gui_str!(ctx.game_state, info_menu_title))
        .pad(Pad::all(sizing::PADDING_LARGE))
        .show(Alignment::TOP_RIGHT, || {
            col_cross_center(|| {
                Text::normal(ctx.game_state.camera.cursor_coord.to_string())
                    .color(colors::TEXT_INACTIVE.yak())
                    .show();

                let id = ctx
                    .game_data
                    .pointing_at
                    .read_latest()
                    .as_ref()
                    .map(|tile| tile.id)
                    .unwrap_or(TileId::none());

                Text::normal(ctx.game_state.resource_man.tile_name(id).to_string()).show();
                GameModel::new(GenericModel::Tile(id), Vec2::splat(sizing::LARGE_ICON_SIZE)).show();

                rest_of_the_info(ctx);
            });
        });
}
