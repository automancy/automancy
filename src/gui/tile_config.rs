use std::time::Instant;

use ractor::rpc::CallResult;
use ractor::ActorRef;

use automancy_defs::{colors::BLACK, coord::TileCoord};
use automancy_defs::{glam::vec2, id::Id};
use automancy_resources::data::inventory::Inventory;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::types::tile::TileDef;
use yakui::{align, column, pad, row, use_state, widgets::Pad, Alignment, Rect, Vec2};

use crate::gui::item::draw_item;
use crate::gui::{info_tip, searchable_id, TextField, MEDIUM_ICON_SIZE, SMALL_ICON_SIZE};
use crate::tile_entity::TileEntityMsg;
use crate::GameState;

use super::components::{
    button::{button, button_text},
    container::{group, window_box},
    interactive::interactive,
    layout::centered_row,
    movable::movable,
    position::PositionRecord,
    slider::slider,
    text::{label, symbol_text},
};

/// Draws the direction selector.
pub fn add_direction(target_coord: &mut Option<TileCoord>, n: u8) {
    let coord = match n {
        0 => Some(TileCoord::TOP_RIGHT),
        1 => Some(TileCoord::RIGHT),
        2 => Some(TileCoord::BOTTOM_RIGHT),
        3 => Some(TileCoord::BOTTOM_LEFT),
        4 => Some(TileCoord::LEFT),
        5 => Some(TileCoord::TOP_LEFT),
        _ => None,
    };

    if button_text(symbol_text(
        match n {
            0 => "\u{f46c}",
            1 => "\u{f432}",
            2 => "\u{f43e}",
            3 => "\u{f424}",
            4 => "\u{f434}",
            5 => "\u{f45c}",
            _ => "",
        },
        BLACK,
    )) // TODO actual selectable value display
    .clicked
    {
        *target_coord = coord;
    }
}

fn config_target(state: &GameState, data: &DataMap, tile_entity: ActorRef<TileEntityMsg>) {
    let current_target_coord = data
        .get(&state.resource_man.registry.data_ids.target)
        .cloned()
        .and_then(Data::into_coord);

    let mut new_target_coord = current_target_coord;
    label(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.tile_config_target]
            .as_str(),
    );

    column(|| {
        row(|| {
            pad(Pad::horizontal(15.0), || {
                add_direction(&mut new_target_coord, 5);
                add_direction(&mut new_target_coord, 0);
            });
        });

        row(|| {
            add_direction(&mut new_target_coord, 4);
            if button("\u{f467}") // TODO actual selectable value display
                .clicked
            {
                new_target_coord = None;
            }
            add_direction(&mut new_target_coord, 1);
        });

        row(|| {
            pad(Pad::horizontal(15.0), || {
                add_direction(&mut new_target_coord, 3);
                add_direction(&mut new_target_coord, 2);
            });
        });
    });

    if new_target_coord != current_target_coord {
        if let Some(target_coord) = new_target_coord {
            tile_entity
                .send_message(TileEntityMsg::SetDataValue(
                    state.resource_man.registry.data_ids.target,
                    Data::Coord(target_coord),
                ))
                .unwrap();
        } else {
            tile_entity
                .send_message(TileEntityMsg::RemoveData(
                    state.resource_man.registry.data_ids.target,
                ))
                .unwrap();
        }
    }
}

fn config_linking(state: &mut GameState, config_open: TileCoord) {
    // TODO make this more generic and not constrained to master_node

    if button(
        &state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.btn_link_network],
    )
    .clicked
    {
        state.gui_state.linking_tile = Some(config_open);
    };

    label(
        &state.resource_man.translates.gui
            [&state.resource_man.registry.gui_ids.lbl_link_destination],
    );
}

fn config_amount(
    state: &mut GameState,
    data: &DataMap,
    tile_entity: ActorRef<TileEntityMsg>,
    tile_info: &TileDef,
) {
    let Some(Data::Amount(max_amount)) = tile_info
        .data
        .get(&state.resource_man.registry.data_ids.max_amount)
        .cloned()
    else {
        return;
    };

    let Data::Amount(current_amount) = data
        .get(&state.resource_man.registry.data_ids.amount)
        .cloned()
        .unwrap_or(Data::Amount(0))
    else {
        return;
    };

    let mut new_amount = current_amount;

    column(|| {
        centered_row(|| {
            label(
                &state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.lbl_amount],
            );
            label(&new_amount.to_string());
        });

        slider(&mut new_amount, 0..=max_amount, Some(128));
    });

    if new_amount != current_amount {
        tile_entity
            .send_message(TileEntityMsg::SetDataValue(
                state.resource_man.registry.data_ids.amount,
                Data::Amount(new_amount),
            ))
            .unwrap();
    }
}

fn takeable_item(
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
        let item = *state.resource_man.registry.items.get(&id).unwrap();

        let pos = use_state(|| Vec2::ZERO);

        if interactive(|| {
            pos.set(
                PositionRecord::new()
                    .show(|| {
                        draw_item(
                            &state.resource_man,
                            None,
                            ItemStack { item, amount },
                            MEDIUM_ICON_SIZE,
                            true,
                        );
                    })
                    .into_inner(),
            )
        })
        .clicked
        {
            if let Some(amount) = buffer.take(id, amount) {
                dirty = true;
                inventory.add(id, amount);

                state
                    .renderer
                    .take_item_animations
                    .entry(item)
                    .or_default()
                    .push_back((
                        Instant::now(),
                        Rect::from_pos_size(pos.get(), vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE)),
                    ));
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

fn config_item(
    state: &mut GameState,
    data: &DataMap,
    item_type: Id,
    tile_entity: ActorRef<TileEntityMsg>,
    tile_info: &TileDef,
) {
    let current_item = data
        .get(&state.resource_man.registry.data_ids.item)
        .cloned()
        .and_then(Data::into_id);
    let mut new_item = current_item;

    let items = state
        .resource_man
        .get_items(item_type, &mut state.loop_store.tag_cache)
        .iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();

    centered_row(|| {
        label(
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.tile_config_item]
                .as_str(),
        );

        if let Some(stack) = current_item
            .and_then(|id| state.resource_man.registry.items.get(&id).cloned())
            .map(|item| ItemStack { item, amount: 0 })
        {
            draw_item(&state.resource_man, None, stack, SMALL_ICON_SIZE, true);
        }
    });

    config_amount(state, data, tile_entity.clone(), tile_info);

    searchable_id(
        items.as_slice(),
        &mut new_item,
        TextField::Filter,
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.search_item_tip]
            .to_string(),
        &|state, id| state.resource_man.item_name(id).to_string(),
        &|state, id| {
            draw_item(
                &state.resource_man,
                None,
                ItemStack {
                    item: state.resource_man.registry.items[id],
                    amount: 0,
                },
                SMALL_ICON_SIZE,
                true,
            );
        },
        state,
    );

    if new_item != current_item {
        if let Some(item) = new_item {
            tile_entity
                .send_message(TileEntityMsg::SetDataValue(
                    state.resource_man.registry.data_ids.item,
                    Data::Id(item),
                ))
                .unwrap();
            tile_entity
                .send_message(TileEntityMsg::RemoveData(
                    state.resource_man.registry.data_ids.buffer,
                ))
                .unwrap();
        }
    }
}

fn draw_script_info(state: &mut GameState, script: Option<Id>) {
    let Some(script) = script.and_then(|id| state.resource_man.registry.scripts.get(&id)) else {
        return;
    };

    column(|| {
        if let Some(inputs) = &script.instructions.inputs {
            for input in inputs {
                draw_item(
                    &state.resource_man,
                    Some(" + "),
                    *input,
                    SMALL_ICON_SIZE,
                    true,
                );
            }
        }

        for output in &script.instructions.outputs {
            draw_item(
                &state.resource_man,
                Some("=> "),
                *output,
                SMALL_ICON_SIZE,
                true,
            );
        }
    });
}

fn config_script(
    state: &mut GameState,
    data: &DataMap,
    scripts: &[Id],
    tile_entity: ActorRef<TileEntityMsg>,
) {
    let current_script = data
        .get(&state.resource_man.registry.data_ids.script)
        .cloned()
        .and_then(Data::into_id);

    centered_row(|| {
        label(
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.tile_config_script]
                .as_str(),
        );

        info_tip(
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.tile_config_script_info]
                .as_str(),
        );
    });

    draw_script_info(state, current_script);

    let mut new_script = current_script;

    searchable_id(
        scripts,
        &mut new_script,
        TextField::Filter,
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.search_script_tip]
            .to_string(),
        &|state, id| state.resource_man.script_name(id).to_string(),
        &|state, id| {
            if let Some(stacks) = state
                .resource_man
                .registry
                .scripts
                .get(id)
                .map(|script| script.instructions.outputs.as_slice())
            {
                for stack in stacks {
                    draw_item(&state.resource_man, None, *stack, SMALL_ICON_SIZE, true);
                }
            }
        },
        state,
    );

    if new_script != current_script {
        if let Some(script) = new_script {
            tile_entity
                .send_message(TileEntityMsg::SetDataValue(
                    state.resource_man.registry.data_ids.script,
                    Data::Id(script),
                ))
                .unwrap();
            tile_entity
                .send_message(TileEntityMsg::RemoveData(
                    state.resource_man.registry.data_ids.buffer,
                ))
                .unwrap();
        }
    }
}

/// Draws the tile configuration menu.
pub fn tile_config_ui(state: &mut GameState, game_data: &mut DataMap) {
    let Some(config_open_at) = state.gui_state.config_open_at else {
        return;
    };

    let Some((tile, entity)) = state.loop_store.config_open_cache.blocking_lock().clone() else {
        return;
    };

    let Ok(CallResult::Success(data)) = state
        .tokio
        .block_on(entity.call(TileEntityMsg::GetData, None))
    else {
        return;
    };

    movable(|| {
        window_box(
            state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.tile_config]
                .to_string(),
            || {
                column(|| {
                    let tile_info = state
                        .resource_man
                        .registry
                        .tiles
                        .get(&tile)
                        .unwrap()
                        .clone();

                    if let Some(Data::VecId(scripts)) = tile_info
                        .data
                        .get(&state.resource_man.registry.data_ids.scripts)
                    {
                        config_script(state, &data, scripts, entity.clone());
                    }

                    if tile_info
                        .data
                        .get(&state.resource_man.registry.data_ids.storage_takeable)
                        .cloned()
                        .and_then(Data::into_bool)
                        .unwrap_or(false)
                    {
                        if let Some(Data::Inventory(buffer)) = data
                            .get(&state.resource_man.registry.data_ids.buffer)
                            .cloned()
                        {
                            centered_row(|| {
                                label(
                                    &state.resource_man.translates.gui
                                        [&state.resource_man.registry.gui_ids.inventory],
                                );

                                info_tip(
                                    &state.resource_man.translates.gui
                                        [&state.resource_man.registry.gui_ids.inventory_tip],
                                );
                            });

                            group(|| {
                                takeable_item(state, game_data, buffer, entity.clone());
                            });
                        }
                    }

                    if let Some(Data::Id(item_type)) = tile_info
                        .data
                        .get(&state.resource_man.registry.data_ids.item_type)
                        .cloned()
                    {
                        config_item(state, &data, item_type, entity.clone(), &tile_info);
                    }

                    if !tile_info
                        .data
                        .get(&state.resource_man.registry.data_ids.not_targeted)
                        .cloned()
                        .and_then(Data::into_bool)
                        .unwrap_or(false)
                    {
                        config_target(state, &data, entity.clone());
                    }

                    if tile_info
                        .data
                        .get(&state.resource_man.registry.data_ids.linking)
                        .cloned()
                        .and_then(Data::into_bool)
                        .unwrap_or(false)
                    {
                        config_linking(state, config_open_at);
                    }
                });
            },
        );
    });
}
