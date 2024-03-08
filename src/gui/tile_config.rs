use std::time::Instant;

use egui::{vec2, DragValue, Margin, Ui, Window};
use egui::{Context, Frame};
use ractor::rpc::CallResult;
use ractor::ActorRef;
use tokio::runtime::Runtime;

use automancy_defs::coord::TileCoord;
use automancy_defs::id::Id;
use automancy_defs::math::Float;
use automancy_resources::data::inventory::Inventory;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::types::tile::TileDef;
use automancy_resources::ResourceManager;

use crate::event::EventLoopStorage;
use crate::gui::item::draw_item;
use crate::gui::{info_hover, TextField, MEDIUM_ICON_SIZE, SMALL_ICON_SIZE};
use crate::setup::GameSetup;
use crate::tile_entity::TileEntityMsg;

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
            0 => "↗",
            1 => "➡",
            2 => "↘",
            3 => "↙",
            4 => "⬅",
            5 => "↖",
            _ => "",
        },
    );
}

fn config_target(
    ui: &mut Ui,
    setup: &GameSetup,
    data: &DataMap,
    tile_entity: ActorRef<TileEntityMsg>,
) {
    let current_target_coord = data
        .get(&setup.resource_man.registry.data_ids.target)
        .cloned()
        .and_then(Data::into_coord);

    let mut new_target_coord = current_target_coord;
    ui.label(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.tile_config_target]
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
            ui.selectable_value(&mut new_target_coord, None, "❌");
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
                    setup.resource_man.registry.data_ids.target,
                    Data::Coord(target_coord),
                ))
                .unwrap();
        } else {
            tile_entity
                .send_message(TileEntityMsg::RemoveData(
                    setup.resource_man.registry.data_ids.target,
                ))
                .unwrap();
        }
    }
}

fn config_linking(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    config_open: TileCoord,
) {
    // TODO make this more generic and not constrained to master_node

    if ui
        .button(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.btn_link_network]
                .to_string(),
        )
        .clicked()
    {
        loop_store.linking_tile = Some(config_open);
    };

    ui.label(
        setup.resource_man.translates.gui
            [&setup.resource_man.registry.gui_ids.lbl_link_destination]
            .to_string(),
    );
}

fn config_amount(
    ui: &mut Ui,
    setup: &GameSetup,
    data: &DataMap,
    tile_entity: ActorRef<TileEntityMsg>,
    tile_info: &TileDef,
) {
    let Some(Data::Amount(max_amount)) = tile_info
        .data
        .get(&setup.resource_man.registry.data_ids.max_amount)
        .cloned()
    else {
        return;
    };

    let Data::Amount(current_amount) = data
        .get(&setup.resource_man.registry.data_ids.amount)
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
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.lbl_amount]
                    .to_string(),
            ),
    );

    if new_amount != current_amount {
        tile_entity
            .send_message(TileEntityMsg::SetDataValue(
                setup.resource_man.registry.data_ids.amount,
                Data::Amount(new_amount),
            ))
            .unwrap();
    }
}

fn takeable_item(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    mut buffer: Inventory,
    game_data: &mut DataMap,
    tile_entity: ActorRef<TileEntityMsg>,
) {
    let Data::Inventory(inventory) = game_data
        .entry(setup.resource_man.registry.data_ids.player_inventory)
        .or_insert_with(|| Data::Inventory(Default::default()))
    else {
        return;
    };

    let mut dirty = false;

    for (id, amount) in buffer.clone().into_inner() {
        let item = *setup.resource_man.registry.items.get(&id).unwrap();

        let (rect, response) = draw_item(
            ui,
            &setup.resource_man,
            None,
            ItemStack { item, amount },
            MEDIUM_ICON_SIZE,
            true,
        );

        if response.clicked() {
            if let Some(amount) = buffer.take(id, amount) {
                dirty = true;
                inventory.add(id, amount);
                loop_store
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
                setup.resource_man.registry.data_ids.buffer,
                Data::Inventory(buffer),
            ))
            .unwrap();
    }
}

fn config_item(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    data: &DataMap,
    item_type: Id,
    tile_entity: ActorRef<TileEntityMsg>,
    tile_info: &TileDef,
) {
    let current_item = data
        .get(&setup.resource_man.registry.data_ids.item)
        .cloned()
        .and_then(Data::into_id);
    let mut new_item = current_item;

    let items = setup
        .resource_man
        .get_items(item_type, &mut loop_store.tag_cache)
        .iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();

    ui.horizontal(|ui| {
        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.tile_config_item]
                .as_str(),
        );

        config_amount(ui, setup, data, tile_entity.clone(), tile_info);
    });

    if let Some(stack) = current_item
        .and_then(|id| setup.resource_man.registry.items.get(&id).cloned())
        .map(|item| ItemStack { item, amount: 0 })
    {
        draw_item(ui, &setup.resource_man, None, stack, SMALL_ICON_SIZE, true);
    }
    loop_store.gui_state.text_field.searchable_id(
        ui,
        &setup.resource_man,
        items.as_slice(),
        &mut new_item,
        TextField::Filter,
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.hint_search_item]
            .to_string(),
        &ResourceManager::item_name,
        &|ui, resource_man, id| {
            draw_item(
                ui,
                resource_man,
                None,
                ItemStack {
                    item: resource_man.registry.items[id],
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
                    setup.resource_man.registry.data_ids.item,
                    Data::Id(item),
                ))
                .unwrap();
            tile_entity
                .send_message(TileEntityMsg::RemoveData(
                    setup.resource_man.registry.data_ids.buffer,
                ))
                .unwrap();
        }
    }
}

fn draw_script_info(ui: &mut Ui, setup: &GameSetup, script: Option<Id>) {
    let Some(script) = script.and_then(|id| setup.resource_man.registry.scripts.get(&id)) else {
        return;
    };

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

        if let Some(inputs) = &script.instructions.inputs {
            for input in inputs {
                draw_item(
                    ui,
                    &setup.resource_man,
                    Some(" + "),
                    *input,
                    SMALL_ICON_SIZE,
                    true,
                );
            }
        }

        for output in &script.instructions.outputs {
            draw_item(
                ui,
                &setup.resource_man,
                Some("=> "),
                *output,
                SMALL_ICON_SIZE,
                true,
            );
        }
    });
}

fn config_script(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    data: &DataMap,
    scripts: &[Id],
    tile_entity: ActorRef<TileEntityMsg>,
) {
    let current_script = data
        .get(&setup.resource_man.registry.data_ids.script)
        .cloned()
        .and_then(Data::into_id);

    ui.horizontal(|ui| {
        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.tile_config_script]
                .as_str(),
        );
        info_hover(
            ui,
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.tile_config_script_info]
                .as_str(),
        );
    });

    draw_script_info(ui, setup, current_script);

    let mut new_script = current_script;

    loop_store.gui_state.text_field.searchable_id(
        ui,
        &setup.resource_man,
        scripts,
        &mut new_script,
        TextField::Filter,
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.hint_search_script]
            .to_string(),
        &ResourceManager::script_name,
        &|ui, resource_man, id| {
            if let Some(stacks) = resource_man
                .registry
                .scripts
                .get(id)
                .map(|script| script.instructions.outputs.as_slice())
            {
                for stack in stacks {
                    draw_item(ui, resource_man, None, *stack, SMALL_ICON_SIZE, false);
                }
            }
        },
    );

    if new_script != current_script {
        if let Some(script) = new_script {
            tile_entity
                .send_message(TileEntityMsg::SetDataValue(
                    setup.resource_man.registry.data_ids.script,
                    Data::Id(script),
                ))
                .unwrap();
            tile_entity
                .send_message(TileEntityMsg::RemoveData(
                    setup.resource_man.registry.data_ids.buffer,
                ))
                .unwrap();
        }
    }
}

/// Draws the tile configuration menu.
pub fn tile_config(
    runtime: &Runtime,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    context: &Context,
    game_data: &mut DataMap,
) {
    let Some(config_open_at) = loop_store.config_open_at else {
        return;
    };

    let Some((tile, entity)) = loop_store.config_open_cache.blocking_lock().clone() else {
        return;
    };

    let Ok(CallResult::Success(data)) = runtime.block_on(entity.call(TileEntityMsg::GetData, None))
    else {
        return;
    };

    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.tile_config]
            .to_string(),
    )
    .resizable(false)
    .auto_sized()
    .constrain(true)
    .frame(Frame::window(&context.style()).inner_margin(Margin::same(10.0)))
    .show(context, |ui| {
        const MARGIN: Float = 10.0;

        ui.set_max_width(300.0);

        let tile_info = setup.resource_man.registry.tiles.get(&tile).unwrap();

        if let Some(Data::VecId(scripts)) = tile_info
            .data
            .get(&setup.resource_man.registry.data_ids.scripts)
        {
            ui.add_space(MARGIN);
            ui.vertical(|ui| {
                config_script(ui, setup, loop_store, &data, scripts, entity.clone());
            });
            ui.add_space(MARGIN);
        }

        if tile_info
            .data
            .get(&setup.resource_man.registry.data_ids.storage_takeable)
            .cloned()
            .and_then(Data::into_bool)
            .unwrap_or(false)
        {
            if let Some(Data::Inventory(buffer)) = data
                .get(&setup.resource_man.registry.data_ids.buffer)
                .cloned()
            {
                ui.add_space(MARGIN);
                ui.vertical(|ui| {
                    takeable_item(ui, setup, loop_store, buffer, game_data, entity.clone());
                });
                ui.add_space(MARGIN);
            }
        }

        if let Some(Data::Id(item_type)) = tile_info
            .data
            .get(&setup.resource_man.registry.data_ids.item_type)
            .cloned()
        {
            ui.add_space(MARGIN);
            ui.vertical(|ui| {
                config_item(
                    ui,
                    setup,
                    loop_store,
                    &data,
                    item_type,
                    entity.clone(),
                    tile_info,
                );
            });
            ui.add_space(MARGIN);
        }

        if !tile_info
            .data
            .get(&setup.resource_man.registry.data_ids.not_targeted)
            .cloned()
            .and_then(Data::into_bool)
            .unwrap_or(false)
        {
            ui.add_space(MARGIN);
            ui.vertical(|ui| {
                config_target(ui, setup, &data, entity.clone());
            });
            ui.add_space(MARGIN);
        }

        if tile_info
            .data
            .get(&setup.resource_man.registry.data_ids.linking)
            .cloned()
            .and_then(Data::into_bool)
            .unwrap_or(false)
        {
            ui.add_space(MARGIN);
            ui.vertical(|ui| {
                config_linking(ui, setup, loop_store, config_open_at);
            });
            ui.add_space(MARGIN);
        }
    });
}
