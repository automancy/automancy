use automancy_defs::{colors, id::TileId, math::vec2, rendering::InstanceData};
use automancy_resources::{data::DataMap, types::IconMode};
use automancy_ui::{
    LABEL_SIZE, LARGE_ICON_SIZE, PADDING_LARGE, UiGameObjectType, col, col_align_end,
    colored_label, colored_sized_text, group, label, row, ui_game_object, window_box,
};
use winit::keyboard::{Key, NamedKey};
use yakui::{
    Alignment, Dim2, Pivot, reflow,
    widgets::{Layer, Pad},
};

use crate::GameState;

#[track_caller]
fn input_hint_names(state: &mut GameState) {
    for hint in &state.input_hints {
        let name = hint
            .last()
            .and_then(|action| {
                state
                    .input_handler
                    .key_map
                    .values()
                    .find(|v| v.action == *action)
            })
            .and_then(|v| v.name);

        if let Some(name) = name.and_then(|name| state.resource_man.translates.keys.get(&name)) {
            label(name);
        } else {
            label(&state.resource_man.translates.unnamed);
        }
    }
}

#[track_caller]
fn input_hint_keys(state: &mut GameState) {
    for hint in &state.input_hints {
        let hint_text = hint
            .iter()
            .flat_map(|action| {
                if let Some((key, _key_action)) = state
                    .input_handler
                    .key_map
                    .iter()
                    .find(|(_, v)| v.action == *action)
                {
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

        colored_sized_text(&hint_text, colors::GRAY, LABEL_SIZE).show();
    }
}

fn rest_of_the_info(state: &mut GameState) {
    group(|| {
        row(|| {
            col(|| {
                input_hint_names(state);
            });

            col_align_end(|| {
                input_hint_keys(state);
            });
        });
    });
}

fn tile_icon(id: TileId) {
    ui_game_object(
        InstanceData::default(),
        UiGameObjectType::Tile(id, DataMap::default()),
        vec2(LARGE_ICON_SIZE, LARGE_ICON_SIZE),
        Some(IconMode::Tile.model_matrix()),
        Some(IconMode::Tile.world_matrix()),
    );
}

/// Draws the info GUI.
pub fn info_ui(state: &mut GameState) {
    Layer::new().show(|| {
        reflow(Alignment::TOP_RIGHT, Pivot::TOP_RIGHT, Dim2::ZERO, || {
            Pad::all(PADDING_LARGE).show(|| {
                window_box(
                    state
                        .resource_man
                        .gui_str(state.resource_man.registry.gui_ids.info)
                        .to_string(),
                    || {
                        colored_label(&state.camera.pointing_at.to_string(), colors::DARK_GRAY);

                        let Some((tile, _entity)) =
                            state.loop_store.pointing_cache.blocking_lock().clone()
                        else {
                            label(
                                &state
                                    .resource_man
                                    .tile_name(TileId(state.resource_man.registry.none)),
                            );

                            tile_icon(TileId(state.resource_man.registry.none));

                            rest_of_the_info(state);

                            return;
                        };

                        label(&state.resource_man.tile_name(tile));

                        tile_icon(tile);

                        rest_of_the_info(state);
                    },
                );
            });
        });
    });
}
