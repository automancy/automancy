use crate::GameState;
use automancy_defs::id::Id;
use automancy_defs::{colors, coord::TileCoord, stack::ItemStack};
use automancy_resources::rhai_ui::RhaiUiUnit;
use automancy_resources::{
    data::{Data, DataMap},
    inventory::Inventory,
};
use automancy_system::tile_entity::TileEntityMsg;
use automancy_system::ui_state::TextField;
use automancy_ui::{
    button, center_col, center_row, col, group, info_tip, interactive, label, list_col, movable,
    num_input, row, scroll_vertical_bar_alignment, selectable_symbol_button, selection_button,
    slider, spaced_col, spaced_row, symbol, symbol_button, window_box, PositionRecord,
    MEDIUM_ICON_SIZE, PADDING_MEDIUM, PADDING_XSMALL, SMALL_ICON_SIZE,
};
use ractor::rpc::CallResult;
use ractor::ActorRef;
use std::time::Instant;
use yakui::{
    constrained,
    widgets::{Layer, Pad},
    Constraints, Rect, Vec2,
};

use super::item::draw_item;
use super::util::searchable_id;

/// Draws the direction selector.
fn add_direction(target_coord: &mut Option<TileCoord>, n: u8) {
    let coord = match n {
        0 => Some(TileCoord::TOP_RIGHT),
        1 => Some(TileCoord::RIGHT),
        2 => Some(TileCoord::BOTTOM_RIGHT),
        3 => Some(TileCoord::BOTTOM_LEFT),
        4 => Some(TileCoord::LEFT),
        5 => Some(TileCoord::TOP_LEFT),
        _ => None,
    };

    selection_button(target_coord, coord, |selected| {
        selectable_symbol_button(
            match n {
                0 => "\u{f46c}",
                1 => "\u{f432}",
                2 => "\u{f43e}",
                3 => "\u{f424}",
                4 => "\u{f434}",
                5 => "\u{f45c}",
                _ => "",
            },
            colors::BLACK,
            selected,
        )
    });
}

fn takeable_items(
    state: &mut GameState,
    game_data: &mut DataMap,
    mut buffer: Inventory,
    buffer_id: Id,
    tile_entity: ActorRef<TileEntityMsg>,
) {
    let Data::Inventory(inventory) = game_data
        .entry(state.resource_man.registry.data_ids.player_inventory)
        .or_insert_with(|| Data::Inventory(Default::default()))
    else {
        return;
    };

    let mut dirty = false;

    for (id, amount) in buffer.clone().into_inner() {
        let mut pos = None;

        let interact = interactive(|| {
            pos = PositionRecord::new()
                .show(|| {
                    draw_item(
                        &state.resource_man,
                        || {},
                        ItemStack { id, amount },
                        MEDIUM_ICON_SIZE,
                        true,
                    );
                })
                .into_inner();
        });

        if interact.clicked {
            let amount = buffer.take(id, amount);

            if amount > 0 {
                dirty = true;
                inventory.add(id, amount);

                if let Some(pos) = pos {
                    state
                        .renderer
                        .as_mut()
                        .unwrap()
                        .take_item_animations
                        .entry(id)
                        .or_default()
                        .push_back((
                            Instant::now(),
                            Rect::from_pos_size(pos, Vec2::new(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE)),
                        ));
                }
            }
        }
    }

    if dirty {
        tile_entity
            .send_message(TileEntityMsg::SetDataValue(
                buffer_id,
                Data::Inventory(buffer),
            ))
            .unwrap();
    }
}

fn draw_item_plain(state: &mut GameState, id: Id) {
    draw_item(
        &state.resource_man,
        || {},
        ItemStack { id, amount: 0 },
        SMALL_ICON_SIZE,
        true,
    );
}

fn draw_item_script(state: &mut GameState, id: Id) {
    if let Some(stacks) = state
        .resource_man
        .registry
        .scripts
        .get(&id)
        .map(|script| script.instructions.outputs.as_slice())
    {
        for stack in stacks {
            draw_item(&state.resource_man, || {}, *stack, SMALL_ICON_SIZE, false);
        }
    }

    label(&state.resource_man.script_name(id));
}

fn draw_script_info(state: &mut GameState, data: &DataMap, id: Id) {
    let script = data.get(id).cloned().and_then(Data::into_id);

    let Some(script) = script.and_then(|id| state.resource_man.registry.scripts.get(&id)) else {
        return;
    };

    col(|| {
        if let Some(inputs) = &script.instructions.inputs {
            for input in inputs {
                draw_item(
                    &state.resource_man,
                    || symbol("\u{f44d}", colors::INPUT),
                    *input,
                    SMALL_ICON_SIZE,
                    true,
                );
            }
        }

        for output in &script.instructions.outputs {
            draw_item(
                &state.resource_man,
                || symbol("\u{f460}", colors::OUTPUT),
                *output,
                SMALL_ICON_SIZE,
                true,
            );
        }
    });
}

fn rhai_ui(
    state: &mut GameState,
    tile_entity: ActorRef<TileEntityMsg>,
    data: &DataMap,
    game_data: &mut DataMap,
    ui: RhaiUiUnit,
) {
    match ui {
        RhaiUiUnit::Label { id } => {
            label(&state.resource_man.gui_str(id));
        }
        RhaiUiUnit::InfoTip { id } => {
            info_tip(&state.resource_man.gui_str(id));
        }
        RhaiUiUnit::LabelAmount { amount } => {
            label(&amount.to_string());
        }
        RhaiUiUnit::InputAmount { id, max } => {
            let Data::Amount(current_amount) = data.get(id).cloned().unwrap_or(Data::Amount(0))
            else {
                return;
            };

            let mut new_amount = current_amount;
            let max_digit_count = (max.checked_ilog10().unwrap_or(0) + 1) as usize;

            num_input(
                &mut new_amount,
                false,
                0..=max,
                |v| v.parse().ok(),
                |v| {
                    let n = v.to_string();
                    let spaces = " ".repeat(max_digit_count.saturating_sub(n.len()));

                    format!("{spaces}{n}")
                },
            );

            if new_amount != current_amount {
                tile_entity
                    .send_message(TileEntityMsg::SetDataValue(id, Data::Amount(new_amount)))
                    .unwrap();
            }
        }
        RhaiUiUnit::SliderAmount { id, max } => {
            let Data::Amount(current_amount) = data.get(id).cloned().unwrap_or(Data::Amount(0))
            else {
                return;
            };

            let mut new_amount = current_amount;
            let max_digit_count = (max.checked_ilog10().unwrap_or(0) + 1) as usize;

            slider(
                &mut new_amount,
                0..=max,
                None,
                |v| v.parse().ok(),
                |v| {
                    let n = v.to_string();
                    let spaces = " ".repeat(max_digit_count.saturating_sub(n.len()));

                    format!("{spaces}{n}")
                },
            );

            if new_amount != current_amount {
                tile_entity
                    .send_message(TileEntityMsg::SetDataValue(id, Data::Amount(new_amount)))
                    .unwrap();
            }
        }
        RhaiUiUnit::HexDirInput { id } => {
            let current_dir = data.get(id).cloned().and_then(Data::into_coord);

            let mut new_dir = current_dir;

            center_col(|| {
                constrained(Constraints::loose(Vec2::new(70.0, 90.0)), || {
                    spaced_col(|| {
                        spaced_row(|| {
                            add_direction(&mut new_dir, 5);
                            add_direction(&mut new_dir, 0);
                        });

                        spaced_row(|| {
                            add_direction(&mut new_dir, 4);
                            if symbol_button("\u{f467}", colors::RED).clicked {
                                new_dir = None;
                            }
                            add_direction(&mut new_dir, 1);
                        });

                        spaced_row(|| {
                            add_direction(&mut new_dir, 3);
                            add_direction(&mut new_dir, 2);
                        });
                    });
                });
            });

            if new_dir != current_dir {
                if let Some(coord) = new_dir {
                    tile_entity
                        .send_message(TileEntityMsg::SetDataValue(id, Data::Coord(coord)))
                        .unwrap();
                } else {
                    tile_entity
                        .send_message(TileEntityMsg::RemoveData(id))
                        .unwrap();
                }
            }
        }
        RhaiUiUnit::SelectableItems {
            data_id,
            hint_id,
            ids,
        } => {
            let current_id = data.get(data_id).cloned().and_then(Data::into_id);
            let mut new_id = current_id;

            let hint = state.resource_man.gui_str(hint_id);

            searchable_id(
                state,
                &ids,
                &mut new_id,
                TextField::Filter,
                Some(hint),
                draw_item_plain,
                |state, id| state.resource_man.item_name(id),
            );

            if new_id != current_id {
                if let Some(id) = new_id {
                    tile_entity
                        .send_message(TileEntityMsg::SetDataValue(data_id, Data::Id(id)))
                        .unwrap();
                }
            }
        }
        RhaiUiUnit::SelectableScripts {
            data_id,
            hint_id,
            ids,
        } => {
            let current_id = data.get(data_id).cloned().and_then(Data::into_id);
            let mut new_id = current_id;

            let hint = state.resource_man.gui_str(hint_id);

            searchable_id(
                state,
                &ids,
                &mut new_id,
                TextField::Filter,
                Some(hint),
                draw_item_script,
                |state, id| state.resource_man.script_name(id),
            );

            if new_id != current_id {
                if let Some(id) = new_id {
                    tile_entity
                        .send_message(TileEntityMsg::SetDataValue(data_id, Data::Id(id)))
                        .unwrap();
                }
            }

            draw_script_info(state, data, data_id);
        }
        RhaiUiUnit::Inventory { id, empty_text } => {
            col(|| {
                if let Some(Data::Inventory(inventory)) = data.get(id).cloned() {
                    takeable_items(state, game_data, inventory, id, tile_entity.clone());
                } else {
                    label(&state.resource_man.gui_str(empty_text));
                }
            });
        }
        RhaiUiUnit::Linkage { id, button_text } => {
            if button(&state.resource_man.gui_str(button_text)).clicked {
                state.ui_state.linking_tile = state.ui_state.config_open_at.zip(Some(id));
            };
        }
        RhaiUiUnit::Row { e } => {
            row(|| {
                for ui in e {
                    rhai_ui(state, tile_entity.clone(), data, game_data, ui);
                }
            });
        }
        RhaiUiUnit::CenterRow { e } => {
            center_row(|| {
                for ui in e {
                    rhai_ui(state, tile_entity.clone(), data, game_data, ui);
                }
            });
        }
        RhaiUiUnit::Col { e } => {
            {
                let mut col = list_col();
                col.item_spacing = PADDING_XSMALL;
                col
            }
            .show(|| {
                for ui in e {
                    rhai_ui(state, tile_entity.clone(), data, game_data, ui);
                }
            });
        }
    }
}

/// Draws the tile configuration menu.
pub fn tile_config_ui(state: &mut GameState, game_data: &mut DataMap) {
    Layer::new().show(|| {
        let Some(tile_entity) = state.loop_store.config_open_cache.blocking_lock().clone() else {
            return;
        };

        let Ok(CallResult::Success(data)) = state
            .tokio
            .block_on(tile_entity.call(TileEntityMsg::GetData, None))
        else {
            return;
        };

        let tile_config_ui;
        if let Ok(CallResult::Success(ui)) = state
            .tokio
            .block_on(tile_entity.call(TileEntityMsg::GetTileConfigUi, None))
        {
            tile_config_ui = ui;
        } else {
            tile_config_ui = None;
        }

        let mut pos = state.ui_state.tile_config_ui_position;
        movable(&mut pos, || {
            window_box(
                state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.tile_config)
                    .to_string(),
                || {
                    scroll_vertical_bar_alignment(
                        Vec2::ZERO,
                        Vec2::new(f32::INFINITY, 360.0),
                        None,
                        || {
                            group(|| {
                                Pad::horizontal(PADDING_MEDIUM).show(|| {
                                    col(|| {
                                        if let Some(ui) = tile_config_ui {
                                            rhai_ui(
                                                state,
                                                tile_entity.clone(),
                                                &data,
                                                game_data,
                                                ui,
                                            );
                                        }
                                    });
                                });
                            });
                        },
                    );
                },
            );
        });
        state.ui_state.tile_config_ui_position = pos;
    });
}
