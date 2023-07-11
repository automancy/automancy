use egui::Context;
use egui::{vec2, DragValue, Margin, Ui, Window};
use ractor::ActorRef;
use tokio::runtime::Runtime;

use automancy::game::GameMsg;
use automancy::renderer::GuiInstances;
use automancy::tile_entity::TileEntityMsg;
use automancy_defs::coord::{TileCoord, TileHex};
use automancy_defs::hexagon_tiles::traits::HexDirection;
use automancy_defs::id::Id;
use automancy_defs::math::Float;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::tile::Tile;
use automancy_resources::ResourceManager;

use crate::event::EventLoopStorage;
use crate::gui::item::draw_item;
use crate::gui::searchable_id;
use crate::setup::GameSetup;

/// Draws the direction selector.
pub fn add_direction(ui: &mut Ui, target_coord: &mut Option<TileCoord>, n: usize) {
    let coord = TileHex::NEIGHBORS[(n + 2) % 6];
    let coord = Some(coord.into());

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
        .and_then(Data::as_coord)
        .cloned();
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
    tile_info: &Tile,
) {
    let current_amount = data
        .get(&setup.resource_man.registry.data_ids.amount)
        .and_then(Data::as_amount)
        .cloned()
        .unwrap_or(0);
    let mut new_amount = current_amount;

    if let Some(Data::Amount(max_amount)) = tile_info
        .data
        .get(&setup.resource_man.registry.data_ids.max_amount)
        .cloned()
    {
        ui.add(
            DragValue::new(&mut new_amount)
                .clamp_range(0..=max_amount)
                .speed(1.0)
                .prefix(
                    setup.resource_man.translates.gui
                        [&setup.resource_man.registry.gui_ids.lbl_amount]
                        .to_string(),
                ),
        );
    }

    if new_amount != current_amount {
        tile_entity
            .send_message(TileEntityMsg::SetDataValue(
                setup.resource_man.registry.data_ids.amount,
                Data::Amount(new_amount),
            ))
            .unwrap();
    }
}

fn config_item(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    gui_instances: &mut GuiInstances,
    data: &DataMap,
    item_type: Id,
    tile_entity: ActorRef<TileEntityMsg>,
    tile_info: &Tile,
) {
    let current_item = data
        .get(&setup.resource_man.registry.data_ids.item)
        .and_then(Data::as_id)
        .cloned();
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
        .and_then(|id| setup.resource_man.registry.item(id))
        .cloned()
        .map(|item| ItemStack { item, amount: 0 })
    {
        draw_item(&setup.resource_man, ui, gui_instances, None, stack);
    }

    searchable_id(
        ui,
        &setup.resource_man,
        &loop_store.fuse,
        items.as_slice(),
        &mut new_item,
        &mut loop_store.filter_input,
        &ResourceManager::item_name,
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

fn config_script(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    gui_instances: &mut GuiInstances,
    data: &DataMap,
    scripts: &Vec<Id>,
    tile_entity: ActorRef<TileEntityMsg>,
) {
    let current_script = data
        .get(&setup.resource_man.registry.data_ids.script)
        .and_then(Data::as_id)
        .cloned();
    let mut new_script = current_script;

    ui.label(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.tile_config_script]
            .as_str(),
    );

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

        if let Some(script) = new_script.and_then(|id| setup.resource_man.registry.script(id)) {
            if let Some(inputs) = &script.instructions.inputs {
                for input in inputs {
                    draw_item(&setup.resource_man, ui, gui_instances, Some(" + "), *input);
                }
            }

            for output in &script.instructions.outputs {
                draw_item(&setup.resource_man, ui, gui_instances, Some("=> "), *output);
            }
        }
    });

    searchable_id(
        ui,
        &setup.resource_man,
        &loop_store.fuse,
        scripts.as_slice(),
        &mut new_script,
        &mut loop_store.filter_input,
        &ResourceManager::script_name,
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
    gui_instances: &mut GuiInstances,
    context: &Context,
) {
    if let Some(config_open) = loop_store.config_open {
        let tile = runtime
            .block_on(
                setup
                    .game
                    .call(|reply| GameMsg::GetTile(config_open, reply), None),
            )
            .unwrap()
            .unwrap();

        let tile_entity = runtime
            .block_on(
                setup
                    .game
                    .call(|reply| GameMsg::GetTileEntity(config_open, reply), None),
            )
            .unwrap()
            .unwrap();

        if let Some(((id, _), tile_entity)) = tile.zip(tile_entity) {
            let data = runtime
                .block_on(tile_entity.call(TileEntityMsg::GetData, None))
                .unwrap()
                .unwrap();

            Window::new(
                setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.tile_config]
                    .to_string(),
            )
            .resizable(false)
            .auto_sized()
            .constrain(true)
            .frame(setup.frame.inner_margin(Margin::same(10.0)))
            .show(context, |ui| {
                const MARGIN: Float = 10.0;

                ui.set_max_width(300.0);

                let tile_info = setup.resource_man.registry.tile(id).unwrap();

                if let Some(scripts) = tile_info
                    .data
                    .get(&setup.resource_man.registry.data_ids.scripts)
                    .and_then(Data::as_vec_id)
                {
                    ui.add_space(MARGIN);
                    ui.vertical(|ui| {
                        config_script(
                            ui,
                            setup,
                            loop_store,
                            gui_instances,
                            &data,
                            scripts,
                            tile_entity.clone(),
                        );
                    });
                    ui.add_space(MARGIN);
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
                            gui_instances,
                            &data,
                            item_type,
                            tile_entity.clone(),
                            tile_info,
                        );
                    });
                    ui.add_space(MARGIN);
                }

                if !setup
                    .resource_man
                    .registry
                    .tile_data(id, setup.resource_man.registry.data_ids.not_targeted)
                    .and_then(Data::as_bool)
                    .cloned()
                    .unwrap_or(false)
                {
                    ui.add_space(MARGIN);
                    ui.vertical(|ui| {
                        config_target(ui, setup, &data, tile_entity.clone());
                    });
                    ui.add_space(MARGIN);
                }

                if setup
                    .resource_man
                    .registry
                    .tile_data(id, setup.resource_man.registry.data_ids.linking)
                    .and_then(Data::as_bool)
                    .cloned()
                    .unwrap_or(false)
                {
                    ui.add_space(MARGIN);
                    ui.vertical(|ui| {
                        config_linking(ui, setup, loop_store, config_open);
                    });
                    ui.add_space(MARGIN);
                }
            });
        }
    }
}
