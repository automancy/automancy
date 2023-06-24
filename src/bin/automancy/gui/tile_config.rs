use ractor::ActorRef;
use tokio::runtime::Runtime;

use automancy::game::GameMsg;
use automancy::renderer::Renderer;
use automancy::tile_entity::TileEntityMsg;
use automancy::util::render::hex_to_normalized;
use automancy_defs::cg::{DPoint3, Double};
use automancy_defs::cgmath::point2;
use automancy_defs::colors;
use automancy_defs::coord::{TileCoord, TileHex};
use automancy_defs::egui::{vec2, DragValue, Margin, Ui, Window};
use automancy_defs::egui_winit_vulkano::Gui;
use automancy_defs::hexagon_tiles::traits::HexDirection;
use automancy_defs::id::Id;
use automancy_defs::rendering::GameVertex;
use automancy_defs::winit::dpi::PhysicalSize;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::tile::Tile;
use automancy_resources::ResourceManager;

use crate::event::EventLoopStorage;
use crate::gui::item::ItemStackGuiElement;
use crate::gui::{make_line, searchable_id, ITEM_ICON_SIZE, MARGIN};
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

fn target(
    ui: &mut Ui,
    setup: &GameSetup,
    data: &DataMap,
    tile_entity: ActorRef<TileEntityMsg>,
    id: Id,
) {
    let current_target_coord = data
        .get(&setup.resource_man.registry.data_ids.target)
        .and_then(Data::as_coord)
        .cloned();
    let mut new_target_coord = current_target_coord;

    if setup.resource_man.registry.tile(id).unwrap().targeted {
        ui.add_space(MARGIN);

        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.tile_config_target]
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
    }

    if new_target_coord != current_target_coord {
        if let Some(target_coord) = new_target_coord {
            tile_entity
                .send_message(TileEntityMsg::SetDataValue(
                    setup.resource_man.registry.data_ids.target,
                    Data::Coord(target_coord),
                ))
                .unwrap();
            setup
                .game
                .send_message(GameMsg::SignalTilesUpdated)
                .unwrap();
        } else {
            tile_entity
                .send_message(TileEntityMsg::RemoveData(
                    setup.resource_man.registry.data_ids.target,
                ))
                .unwrap();
            setup
                .game
                .send_message(GameMsg::SignalTilesUpdated)
                .unwrap();
        }
    }
}

fn master_node(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    config_open: TileCoord,
    id: Id,
) {
    if id == setup.resource_man.registry.tile_ids.master_node {
        ui.add_space(MARGIN);

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

        ui.add_space(MARGIN);
    }
}

fn node(
    runtime: &Runtime,
    setup: &GameSetup,
    config_open: TileCoord,
    extra_vertices: &mut Vec<GameVertex>,
    window_size: PhysicalSize<u32>,
    id: Id,
) {
    if id == setup.resource_man.registry.tile_ids.node {
        if let Some(tile_entity) = runtime
            .block_on(
                setup
                    .game
                    .call(|reply| GameMsg::GetTileEntity(config_open, reply), None),
            )
            .unwrap()
            .unwrap()
        {
            let result = runtime
                .block_on(tile_entity.call(
                    |reply| {
                        TileEntityMsg::GetDataValue(
                            setup.resource_man.registry.data_ids.link,
                            reply,
                        )
                    },
                    None,
                ))
                .unwrap()
                .unwrap();

            if let Some(link) = result.as_ref().and_then(Data::as_coord) {
                let DPoint3 { x, y, .. } = hex_to_normalized(
                    window_size.width as Double,
                    window_size.height as Double,
                    setup.camera.get_pos(),
                    config_open,
                );
                let a = point2(x, y);

                let DPoint3 { x, y, .. } = hex_to_normalized(
                    window_size.width as Double,
                    window_size.height as Double,
                    setup.camera.get_pos(),
                    config_open + *link,
                );
                let b = point2(x, y);

                extra_vertices.extend_from_slice(&make_line(a, b, colors::RED));
            }
        }
    }
}

fn storage(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    renderer: &Renderer,
    data: &DataMap,
    tile_entity: ActorRef<TileEntityMsg>,
    tile_info: &Tile,
) {
    let current_storage = data
        .get(&setup.resource_man.registry.data_ids.storage)
        .and_then(Data::as_id)
        .cloned();
    let mut new_storage = current_storage;

    let current_amount = data
        .get(&setup.resource_man.registry.data_ids.amount)
        .and_then(Data::as_amount)
        .cloned()
        .unwrap_or(0);
    let mut new_amount = current_amount;

    if let Some(Data::Id(storage_type)) = tile_info
        .data
        .get(&setup.resource_man.registry.data_ids.storage_type)
    {
        let items = setup
            .resource_man
            .get_items(*storage_type, &mut loop_store.tag_cache)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();

        ui.add_space(MARGIN);

        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.tile_config_storage]
                .as_str(),
        );
        ui.horizontal(|ui| {
            ui.set_height(ITEM_ICON_SIZE);

            if let Some(stack) = current_storage
                .and_then(|id| setup.resource_man.registry.item(id))
                .cloned()
                .and_then(|item| {
                    data.get(&setup.resource_man.registry.data_ids.buffer)
                        .and_then(Data::as_inventory)
                        .and_then(|inventory| {
                            inventory
                                .try_get(item)
                                .map(|amount| ItemStack { item, amount })
                        })
                })
            {
                ui.add(ItemStackGuiElement::new(
                    setup.resource_man.clone(),
                    renderer,
                    stack,
                ));
            }
            ui.add(
                DragValue::new(&mut new_amount)
                    .clamp_range(0..=65535)
                    .speed(1.0)
                    .prefix(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.lbl_amount]
                            .to_string(),
                    ),
            );
        });

        ui.add_space(MARGIN);

        searchable_id(
            ui,
            &setup.resource_man,
            &loop_store.fuse,
            items.as_slice(),
            &mut new_storage,
            &mut loop_store.filter_input,
            &ResourceManager::item_name,
        );
    }

    if new_storage != current_storage {
        if let Some(storage) = new_storage {
            tile_entity
                .send_message(TileEntityMsg::SetDataValue(
                    setup.resource_man.registry.data_ids.storage,
                    Data::Id(storage),
                ))
                .unwrap();
            tile_entity
                .send_message(TileEntityMsg::RemoveData(
                    setup.resource_man.registry.data_ids.buffer,
                ))
                .unwrap();
        }
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

fn script(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    renderer: &Renderer,
    data: &DataMap,
    tile_entity: ActorRef<TileEntityMsg>,
    tile_info: &Tile,
) {
    let current_script = data
        .get(&setup.resource_man.registry.data_ids.script)
        .and_then(Data::as_id)
        .cloned();
    let mut new_script = current_script;

    if let Some(scripts) = tile_info
        .data
        .get(&setup.resource_man.registry.data_ids.scripts)
        .and_then(Data::as_vec_id)
    {
        ui.add_space(MARGIN);

        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.tile_config_script]
                .as_str(),
        );

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

            if let Some(script) = new_script.and_then(|id| setup.resource_man.registry.script(id)) {
                if let Some(inputs) = &script.instructions.inputs {
                    inputs.iter().for_each(|stack| {
                        ui.horizontal(|ui| {
                            ui.set_height(ITEM_ICON_SIZE);

                            ui.label(" + ");
                            ui.add(ItemStackGuiElement::new(
                                setup.resource_man.clone(),
                                renderer,
                                *stack,
                            ));
                        });
                    })
                }

                ui.horizontal(|ui| {
                    ui.set_height(ITEM_ICON_SIZE);

                    ui.label("=> ");
                    ui.add(ItemStackGuiElement::new(
                        setup.resource_man.clone(),
                        renderer,
                        script.instructions.output,
                    ));
                });
            }
        });

        ui.add_space(MARGIN);

        searchable_id(
            ui,
            &setup.resource_man,
            &loop_store.fuse,
            scripts.as_slice(),
            &mut new_script,
            &mut loop_store.filter_input,
            &ResourceManager::script_name,
        );
    }

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
    renderer: &Renderer,
    gui: &Gui,
    extra_vertices: &mut Vec<GameVertex>,
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
            .show(&gui.context(), |ui| {
                ui.set_max_width(300.0);

                let window_size = setup.window.inner_size();
                let tile_info = setup.resource_man.registry.tile(id).unwrap();

                script(
                    ui,
                    setup,
                    loop_store,
                    renderer,
                    &data,
                    tile_entity.clone(),
                    tile_info,
                );
                storage(
                    ui,
                    setup,
                    loop_store,
                    renderer,
                    &data,
                    tile_entity.clone(),
                    tile_info,
                );
                target(ui, setup, &data, tile_entity.clone(), id);
                node(runtime, setup, config_open, extra_vertices, window_size, id);
                master_node(ui, setup, loop_store, config_open, id);

                ui.add_space(MARGIN);
            });
        }
    }
}
