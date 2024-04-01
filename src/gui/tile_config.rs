use std::time::Instant;

use egui::Frame;
use egui::{vec2, DragValue, Margin, Ui, Window};
use ractor::rpc::CallResult;
use ractor::ActorRef;

use automancy_defs::coord::TileCoord;
use automancy_defs::id::Id;
use automancy_defs::math::Float;
use automancy_resources::data::inventory::Inventory;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::types::tile::TileDef;

use crate::gui::item::draw_item;
use crate::gui::{hover_tip, searchable_id, TextField, MEDIUM_ICON_SIZE, SMALL_ICON_SIZE};
use crate::tile_entity::TileEntityMsg;
use crate::GameState;

/// Draws the direction selector.
pub fn add_direction(ui: &mut Ui, target_coord: &mut Option<TileCoord>, n: u8) {
    let coord = match n {
        0 => Some(TileCoord::TOP_RIGHT),
        1 => Some(TileCoord::RIGHT),
        2 => Some(TileCoord::BOTTOM_RIGHT),
        3 => Some(TileCoord::BOTTOM_LEFT),
        4 => Some(TileCoord::LEFT),
        5 => Some(TileCoord::TOP_LEFT),
        _ => None,
    };

    ui.selectable_value(
        target_coord,
        coord,
        match n {
            0 => "\u{f46c}",
            1 => "\u{f432}",
            2 => "\u{f43e}",
            3 => "\u{f424}",
            4 => "\u{f434}",
            5 => "\u{f45c}",
            _ => "",
        },
    );
}

fn config_target(
    state: &GameState,
    ui: &mut Ui,
    data: &DataMap,
    tile_entity: ActorRef<TileEntityMsg>,
) {
    let current_target_coord = data
        .get(&state.resource_man.registry.data_ids.target)
        .cloned()
        .and_then(Data::into_coord);

    let mut new_target_coord = current_target_coord;
    ui.label(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.tile_config_target]
            .as_str(),
    );

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.add_space(15.0);
            add_direction(ui, &mut new_target_coord, 5);
            add_direction(ui, &mut new_target_coord, 0);
        });

        ui.horizontal(|ui| {
            add_direction(ui, &mut new_target_coord, 4);
            ui.selectable_value(&mut new_target_coord, None, "\u{f467}");
            add_direction(ui, &mut new_target_coord, 1);
        });

        ui.horizontal(|ui| {
            ui.add_space(15.0);
            add_direction(ui, &mut new_target_coord, 3);
            add_direction(ui, &mut new_target_coord, 2);
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

fn config_linking(state: &mut GameState, ui: &mut Ui, config_open: TileCoord) {
    // TODO make this more generic and not constrained to master_node

    if ui
        .button(
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.btn_link_network]
                .to_string(),
        )
        .clicked()
    {
        state.gui_state.linking_tile = Some(config_open);
    };

    ui.label(
        state.resource_man.translates.gui
            [&state.resource_man.registry.gui_ids.lbl_link_destination]
            .to_string(),
    );
}

fn config_amount(
    state: &mut GameState,
    ui: &mut Ui,
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

    ui.add(
        DragValue::new(&mut new_amount)
            .clamp_range(0..=max_amount)
            .speed(1.0)
            .prefix(
                state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.lbl_amount]
                    .to_string(),
            ),
    );

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
    ui: &mut Ui,
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

        let (rect, response) = draw_item(
            &state.resource_man,
            ui,
            None,
            ItemStack { item, amount },
            MEDIUM_ICON_SIZE,
            true,
        );

        if response.clicked() {
            if let Some(amount) = buffer.take(id, amount) {
                dirty = true;
                inventory.add(id, amount);
                state
                    .renderer
                    .take_item_animations
                    .entry(item)
                    .or_default()
                    .push_back((Instant::now(), rect));
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
    ui: &mut Ui,
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

    ui.horizontal(|ui| {
        ui.label(
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.tile_config_item]
                .as_str(),
        );

        config_amount(state, ui, data, tile_entity.clone(), tile_info);
    });

    if let Some(stack) = current_item
        .and_then(|id| state.resource_man.registry.items.get(&id).cloned())
        .map(|item| ItemStack { item, amount: 0 })
    {
        draw_item(&state.resource_man, ui, None, stack, SMALL_ICON_SIZE, true);
    }

    searchable_id(
        state,
        ui,
        items.as_slice(),
        &mut new_item,
        TextField::Filter,
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.search_item_tip]
            .to_string(),
        &|state, id| state.resource_man.item_name(id).to_string(),
        &|state, ui, id| {
            draw_item(
                &state.resource_man,
                ui,
                None,
                ItemStack {
                    item: state.resource_man.registry.items[id],
                    amount: 0,
                },
                SMALL_ICON_SIZE,
                false,
            );
        },
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

fn draw_script_info(state: &mut GameState, ui: &mut Ui, script: Option<Id>) {
    let Some(script) = script.and_then(|id| state.resource_man.registry.scripts.get(&id)) else {
        return;
    };

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

        if let Some(inputs) = &script.instructions.inputs {
            for input in inputs {
                draw_item(
                    &state.resource_man,
                    ui,
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
                ui,
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
    ui: &mut Ui,
    data: &DataMap,
    scripts: &[Id],
    tile_entity: ActorRef<TileEntityMsg>,
) {
    let current_script = data
        .get(&state.resource_man.registry.data_ids.script)
        .cloned()
        .and_then(Data::into_id);

    ui.horizontal(|ui| {
        ui.label(
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.tile_config_script]
                .as_str(),
        );
        hover_tip(
            ui,
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.tile_config_script_info]
                .as_str(),
        );
    });

    draw_script_info(state, ui, current_script);

    let mut new_script = current_script;

    searchable_id(
        state,
        ui,
        scripts,
        &mut new_script,
        TextField::Filter,
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.search_script_tip]
            .to_string(),
        &|state, id| state.resource_man.script_name(id).to_string(),
        &|state, ui, id| {
            if let Some(stacks) = state
                .resource_man
                .registry
                .scripts
                .get(id)
                .map(|script| script.instructions.outputs.as_slice())
            {
                for stack in stacks {
                    draw_item(
                        &state.resource_man,
                        ui,
                        None,
                        *stack,
                        SMALL_ICON_SIZE,
                        false,
                    );
                }
            }
        },
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

    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.tile_config]
            .to_string(),
    )
    .id("tile_config".into())
    .resizable(false)
    .auto_sized()
    .constrain(true)
    .frame(Frame::window(&state.gui.context.clone().style()).inner_margin(Margin::same(10.0)))
    .show(&state.gui.context.clone(), |ui| {
        const MARGIN: Float = 10.0;

        ui.set_max_width(300.0);

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
            ui.add_space(MARGIN);
            ui.vertical(|ui| {
                config_script(state, ui, &data, scripts, entity.clone());
            });
            ui.add_space(MARGIN);
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
                ui.add_space(MARGIN);
                ui.horizontal(|ui| {
                    ui.label(
                        state.resource_man.translates.gui
                            [&state.resource_man.registry.gui_ids.inventory]
                            .as_str(),
                    );
                    hover_tip(
                        ui,
                        state.resource_man.translates.gui
                            [&state.resource_man.registry.gui_ids.inventory_tip]
                            .as_str(),
                    );
                });
                ui.group(|ui| {
                    takeable_item(state, ui, game_data, buffer, entity.clone());
                });
                ui.add_space(MARGIN);
            }
        }

        if let Some(Data::Id(item_type)) = tile_info
            .data
            .get(&state.resource_man.registry.data_ids.item_type)
            .cloned()
        {
            ui.add_space(MARGIN);
            ui.vertical(|ui| {
                config_item(state, ui, &data, item_type, entity.clone(), &tile_info);
            });
            ui.add_space(MARGIN);
        }

        if !tile_info
            .data
            .get(&state.resource_man.registry.data_ids.not_targeted)
            .cloned()
            .and_then(Data::into_bool)
            .unwrap_or(false)
        {
            ui.add_space(MARGIN);
            ui.vertical(|ui| {
                config_target(state, ui, &data, entity.clone());
            });
            ui.add_space(MARGIN);
        }

        if tile_info
            .data
            .get(&state.resource_man.registry.data_ids.linking)
            .cloned()
            .and_then(Data::into_bool)
            .unwrap_or(false)
        {
            ui.add_space(MARGIN);
            ui.vertical(|ui| {
                config_linking(state, ui, config_open_at);
            });
            ui.add_space(MARGIN);
        }
    });
}
