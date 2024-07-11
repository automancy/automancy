use std::time::Instant;

use ractor::rpc::CallResult;
use ractor::ActorRef;

use automancy_defs::{colors, coord::TileCoord, stack::ItemStack};
use automancy_defs::{glam::vec2, id::Id};
use automancy_resources::rhai_ui::RhaiUiUnit;
use automancy_resources::{
    data::{Data, DataMap},
    inventory::Inventory,
};
use yakui::{constrained, widgets::Layer, Constraints, Vec2};

use crate::tile_entity::TileEntityMsg;
use crate::{gui::row, GameState};

use super::{
    button, center_col, center_row, col, info_tip, interactive, item::draw_item, label, list_col,
    movable, num_input, scroll_vertical_bar_alignment, selectable_symbol_button, selection_button,
    slider, spaced_col, spaced_row, symbol, symbol_button, util::searchable_id, window_box,
    TextField, MEDIUM_ICON_SIZE, PADDING_LARGE, SMALL_ICON_SIZE,
};

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

fn config_direction(state: &GameState, data: &DataMap, tile_entity: ActorRef<TileEntityMsg>) {
    let current_dir = data
        .get(state.resource_man.registry.data_ids.direction)
        .cloned()
        .and_then(Data::into_coord);

    let mut new_dir = current_dir;

    center_row(|| {
        label(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.tile_config_direction),
        );

        info_tip(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.direction_tip),
        );
    });

    center_col(|| {
        constrained(Constraints::loose(vec2(70.0, 90.0)), || {
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
                .send_message(TileEntityMsg::SetDataValue(
                    state.resource_man.registry.data_ids.direction,
                    Data::Coord(coord),
                ))
                .unwrap();
        } else {
            tile_entity
                .send_message(TileEntityMsg::RemoveData(
                    state.resource_man.registry.data_ids.direction,
                ))
                .unwrap();
        }
    }
}

fn config_linking(state: &mut GameState, config_open: TileCoord) {
    // TODO make this more generic and not constrained to master_node

    center_row(|| {
        if button(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.btn_link_network),
        )
        .clicked
        {
            state.gui_state.linking_tile = Some(config_open);
        };

        info_tip(
            &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.link_destination_tip),
        );
    });
}

fn takeable_items(
    state: &mut GameState,
    game_data: &mut DataMap,
    mut buffer: Inventory,
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
        let mut rect = None;

        let interact = interactive(|| {
            rect = draw_item(
                &state.resource_man,
                || {},
                ItemStack { id, amount },
                MEDIUM_ICON_SIZE,
                true,
            );
        });

        if interact.clicked {
            let amount = buffer.take(id, amount);

            if amount > 0 {
                dirty = true;
                inventory.add(id, amount);

                if let Some(rect) = rect {
                    state
                        .renderer
                        .as_mut()
                        .unwrap()
                        .take_item_animations
                        .entry(id)
                        .or_default()
                        .push_back((Instant::now(), rect));
                }
            }
        }
    }

    if dirty {
        tile_entity
            .send_message(TileEntityMsg::SetDataValue(
                state.resource_man.registry.data_ids.buffer,
                Data::Inventory(buffer),
            ))
            .unwrap();
    }
}

fn config_selectable_id(
    state: &mut GameState,
    data: &DataMap,
    data_id: Id,
    hint_id: Id,
    ids: &[Id],
    names: &[impl AsRef<str>],
    tile_entity: ActorRef<TileEntityMsg>,
    draw: &'static impl Fn(&mut GameState, &Id, &str),
) -> bool {
    let current_id = data.get(data_id).cloned().and_then(Data::into_id);
    let mut new_id = current_id;

    searchable_id(
        ids,
        names,
        &mut new_id,
        TextField::Filter,
        &state.resource_man.gui_str(hint_id),
        draw,
        state,
    );

    if new_id != current_id {
        if let Some(id) = new_id {
            tile_entity
                .send_message(TileEntityMsg::SetDataValue(data_id, Data::Id(id)))
                .unwrap();
            return true;
        }
    }

    false
}

fn draw_script_info(state: &mut GameState, data: &mut DataMap) {
    let script = data
        .get(state.resource_man.registry.data_ids.script)
        .cloned()
        .and_then(Data::into_id);

    let Some(script) = script.and_then(|id| state.resource_man.registry.scripts.get(&id)) else {
        return;
    };

    scroll_vertical_bar_alignment(Vec2::ZERO, Vec2::new(f32::INFINITY, 200.0), None, || {
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
    });
}

fn rhai_ui(
    state: &mut GameState,
    tile_entity: ActorRef<TileEntityMsg>,
    data: &mut DataMap,
    ui: RhaiUiUnit,
) {
    match ui {
        RhaiUiUnit::Label { id } => {
            label(&state.resource_man.gui_str(id));
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
        RhaiUiUnit::Row { e } => {
            row(|| {
                for ui in e {
                    rhai_ui(state, tile_entity.clone(), data, ui);
                }
            });
        }
        RhaiUiUnit::Col { e } => {
            col(|| {
                for ui in e {
                    rhai_ui(state, tile_entity.clone(), data, ui);
                }
            });
        }
    }
}

/// Draws the tile configuration menu.
pub fn tile_config_ui(state: &mut GameState, game_data: &mut DataMap) {
    Layer::new().show(|| {
        let Some(config_open_at) = state.gui_state.config_open_at else {
            return;
        };

        let Some((tile, tile_entity)) = state.loop_store.config_open_cache.blocking_lock().clone()
        else {
            return;
        };

        let Ok(CallResult::Success(mut data)) = state
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

        let mut pos = state.gui_state.tile_config_ui_position;
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
                            {
                                let mut col = list_col();
                                col.item_spacing = PADDING_LARGE;
                                col
                            }
                            .show(|| {
                                let tile_info = state
                                    .resource_man
                                    .registry
                                    .tiles
                                    .get(&tile)
                                    .unwrap()
                                    .clone();

                                if tile_info
                                    .data
                                    .get(state.resource_man.registry.data_ids.storage_takeable)
                                    .cloned()
                                    .and_then(Data::into_bool)
                                    .unwrap_or(false)
                                {
                                    if let Some(Data::Inventory(buffer)) = data
                                        .get(state.resource_man.registry.data_ids.buffer)
                                        .cloned()
                                    {
                                        col(|| {
                                            center_row(|| {
                                                label(&state.resource_man.gui_str(
                                                    state.resource_man.registry.gui_ids.inventory,
                                                ));

                                                info_tip(
                                                    &state.resource_man.gui_str(
                                                        state
                                                            .resource_man
                                                            .registry
                                                            .gui_ids
                                                            .inventory_tip,
                                                    ),
                                                );
                                            });

                                            takeable_items(
                                                state,
                                                game_data,
                                                buffer,
                                                tile_entity.clone(),
                                            );
                                        });
                                    }
                                }

                                if let Some(ui) = tile_config_ui {
                                    col(|| {
                                        rhai_ui(state, tile_entity.clone(), &mut data, ui);
                                    });
                                }

                                if let Some(Data::VecId(scripts)) = tile_info
                                    .data
                                    .get(state.resource_man.registry.data_ids.scripts)
                                {
                                    col(|| {
                                        center_row(|| {
                                            label(
                                                &state.resource_man.gui_str(
                                                    state
                                                        .resource_man
                                                        .registry
                                                        .gui_ids
                                                        .tile_config_script,
                                                ),
                                            );

                                            info_tip(
                                                &state.resource_man.gui_str(
                                                    state
                                                        .resource_man
                                                        .registry
                                                        .gui_ids
                                                        .tile_config_script_info,
                                                ),
                                            );
                                        });

                                        draw_script_info(state, &mut data);

                                        let names = scripts
                                            .iter()
                                            .cloned()
                                            .map(|id| state.resource_man.script_name(id))
                                            .collect::<Vec<_>>();

                                        static DRAW: fn(&mut GameState, &Id, &str) =
                                            |state, id, name| {
                                                if let Some(stacks) =
                                                    state.resource_man.registry.scripts.get(id).map(
                                                        |script| {
                                                            script.instructions.outputs.as_slice()
                                                        },
                                                    )
                                                {
                                                    for stack in stacks {
                                                        draw_item(
                                                            &state.resource_man,
                                                            || {},
                                                            *stack,
                                                            SMALL_ICON_SIZE,
                                                            false,
                                                        );
                                                    }
                                                }

                                                label(name);
                                            };

                                        if config_selectable_id(
                                            state,
                                            &data,
                                            state.resource_man.registry.data_ids.script,
                                            state.resource_man.registry.gui_ids.search_script_tip,
                                            scripts,
                                            &names,
                                            tile_entity.clone(),
                                            &DRAW,
                                        ) {
                                            tile_entity
                                                .send_message(TileEntityMsg::RemoveData(
                                                    state.resource_man.registry.data_ids.buffer,
                                                ))
                                                .unwrap();
                                        }
                                    });
                                }

                                if let Some(Data::Id(item_type)) = tile_info
                                    .data
                                    .get(state.resource_man.registry.data_ids.item_type)
                                    .cloned()
                                {
                                    col(|| {
                                        center_row(|| {
                                            label(
                                                &state.resource_man.gui_str(
                                                    state
                                                        .resource_man
                                                        .registry
                                                        .gui_ids
                                                        .tile_config_item_type,
                                                ),
                                            );

                                            let current_id = data
                                                .get(state.resource_man.registry.data_ids.item)
                                                .cloned()
                                                .and_then(Data::into_id);

                                            if let Some(id) = current_id {
                                                draw_item(
                                                    &state.resource_man,
                                                    || {},
                                                    ItemStack { id, amount: 0 },
                                                    SMALL_ICON_SIZE,
                                                    true,
                                                );
                                            }
                                        });

                                        let ids = state
                                            .resource_man
                                            .get_items(item_type, &mut state.loop_store.tag_cache)
                                            .iter()
                                            .map(|item| item.id)
                                            .collect::<Vec<_>>();
                                        let names = ids
                                            .iter()
                                            .cloned()
                                            .map(|id| state.resource_man.item_name(id))
                                            .collect::<Vec<_>>();

                                        static DRAW: fn(&mut GameState, &Id, &str) =
                                            |state, id, _name| {
                                                draw_item(
                                                    &state.resource_man,
                                                    || {},
                                                    ItemStack { id: *id, amount: 0 },
                                                    SMALL_ICON_SIZE,
                                                    true,
                                                );
                                            };

                                        if config_selectable_id(
                                            state,
                                            &data,
                                            state.resource_man.registry.data_ids.item,
                                            state.resource_man.registry.gui_ids.search_item_tip,
                                            &ids,
                                            &names,
                                            tile_entity.clone(),
                                            &DRAW,
                                        ) {
                                            tile_entity
                                                .send_message(TileEntityMsg::RemoveData(
                                                    state.resource_man.registry.data_ids.buffer,
                                                ))
                                                .unwrap();
                                        }
                                    });
                                }

                                if !tile_info
                                    .data
                                    .get(state.resource_man.registry.data_ids.indirectional)
                                    .cloned()
                                    .and_then(Data::into_bool)
                                    .unwrap_or(false)
                                {
                                    col(|| {
                                        config_direction(state, &data, tile_entity.clone());
                                    });
                                }

                                if tile_info
                                    .data
                                    .get(state.resource_man.registry.data_ids.linking)
                                    .cloned()
                                    .and_then(Data::into_bool)
                                    .unwrap_or(false)
                                {
                                    col(|| {
                                        config_linking(state, config_open_at);
                                    });
                                }
                            });
                        },
                    );
                },
            );
        });
        state.gui_state.tile_config_ui_position = pos;
    });
}
