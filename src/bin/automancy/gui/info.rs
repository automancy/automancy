use std::time::Instant;

use egui::{vec2, Align, Align2, Context, Margin, Window};
use tokio::runtime::Runtime;

use automancy::game::{GameMsg, TAKE_ITEM_ANIMATION_SPEED};
use automancy::tile_entity::TileEntityMsg;
use automancy_defs::colors;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::math::Float;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::Data;

use crate::event::EventLoopStorage;
use crate::gui::default_frame;
use crate::gui::item::{draw_item, paint_item, SMALL_ITEM_ICON_SIZE};
use crate::renderer::GuiInstances;
use crate::setup::GameSetup;

/// Draws the info GUI.
pub fn info(
    runtime: &Runtime,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    gui_instances: &mut GuiInstances,
    context: &Context,
) {
    const MARGIN: Float = 30.0;

    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.info].as_str(),
    )
    .anchor(Align2([Align::RIGHT, Align::TOP]), vec2(-10.0, 10.0))
    .resizable(false)
    .default_width(300.0)
    .frame(default_frame().inner_margin(Margin::same(10.0)))
    .show(context, |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.colored_label(colors::DARK_GRAY, setup.camera.pointing_at.to_string());

                let tile_entity = runtime
                    .block_on(setup.game.call(
                        |reply| GameMsg::GetTileEntity(setup.camera.pointing_at, reply),
                        None,
                    ))
                    .unwrap()
                    .unwrap();

                let tile = runtime
                    .block_on(setup.game.call(
                        |reply| GameMsg::GetTile(setup.camera.pointing_at, reply),
                        None,
                    ))
                    .unwrap()
                    .unwrap();

                if let Some((tile_entity, (id, _))) = tile_entity.zip(tile) {
                    ui.label(setup.resource_man.tile_name(&id));

                    let data = runtime
                        .block_on(tile_entity.call(TileEntityMsg::GetData, None))
                        .unwrap()
                        .unwrap();

                    if let Some(inventory) = data
                        .get(&setup.resource_man.registry.data_ids.buffer)
                        .and_then(Data::as_inventory)
                        .cloned()
                    {
                        for (item, amount) in inventory.0.iter().flat_map(|(id, amount)| {
                            setup
                                .resource_man
                                .registry
                                .item(*id)
                                .map(|item| (*item, *amount))
                        }) {
                            draw_item(
                                &setup.resource_man,
                                ui,
                                gui_instances,
                                None,
                                ItemStack { item, amount },
                                SMALL_ITEM_ICON_SIZE,
                            );
                        }
                    }
                    //ui.label(format!("State: {}", ask(sys, &game, )))
                }
            });

            ui.add_space(MARGIN);

            ui.vertical(|ui| {
                if let Some(Data::Inventory(inventory)) = runtime
                    .block_on(setup.game.call(
                        |reply| {
                            GameMsg::GetDataValue(
                                setup.resource_man.registry.data_ids.player_inventory,
                                reply,
                            )
                        },
                        None,
                    ))
                    .unwrap()
                    .unwrap()
                {
                    ui.label(
                        setup.resource_man.translates.gui
                            [&setup.resource_man.registry.gui_ids.player_inventory]
                            .as_str(),
                    );

                    ui.vertical(|ui| {
                        for (item, amount) in inventory.0.iter().flat_map(|(id, amount)| {
                            setup
                                .resource_man
                                .registry
                                .item(*id)
                                .map(|item| (*item, *amount))
                        }) {
                            let (dst_rect, _) = draw_item(
                                &setup.resource_man,
                                ui,
                                gui_instances,
                                None,
                                ItemStack { item, amount },
                                SMALL_ITEM_ICON_SIZE,
                            );

                            let now = Instant::now();

                            let mut to_remove = HashMap::new();

                            for (coord, deque) in &loop_store.take_item_animations {
                                to_remove.insert(
                                    *coord,
                                    deque
                                        .iter()
                                        .take_while(|(instant, _)| {
                                            now.duration_since(*instant)
                                                >= TAKE_ITEM_ANIMATION_SPEED
                                        })
                                        .count(),
                                );
                            }

                            for (coord, v) in to_remove {
                                for _ in 0..v {
                                    loop_store
                                        .take_item_animations
                                        .get_mut(&coord)
                                        .unwrap()
                                        .pop_front();
                                }
                            }

                            if let Some(animations) = loop_store.take_item_animations.get(&item) {
                                for (instant, src_rect) in animations {
                                    let d = now.duration_since(*instant).as_secs_f32()
                                        / TAKE_ITEM_ANIMATION_SPEED.as_secs_f32();

                                    paint_item(
                                        &setup.resource_man,
                                        gui_instances,
                                        item,
                                        src_rect.lerp_towards(&dst_rect, d),
                                    );
                                }
                            }
                        }
                    });
                }
            });
        });
    });
}
