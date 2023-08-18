use std::time::Instant;

use egui::{Context, Rect, ScrollArea, Window};
use tokio::runtime::Runtime;

use automancy::game::{GameMsg, TAKE_ITEM_ANIMATION_SPEED};
use automancy_defs::hashbrown::HashMap;
use automancy_resources::data::item::Item;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::Data;

use crate::event::EventLoopStorage;
use crate::gui::item::{draw_item, paint_item, MEDIUM_ITEM_ICON_SIZE};
use crate::gui::{default_frame, GuiState};
use crate::renderer::GuiInstances;
use crate::setup::GameSetup;

fn take_item_animation(
    item: Item,
    dst_rect: Rect,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    gui_instances: &mut GuiInstances,
) {
    let now = Instant::now();

    let mut to_remove = HashMap::new();

    for (coord, deque) in &loop_store.take_item_animations {
        to_remove.insert(
            *coord,
            deque
                .iter()
                .take_while(|(instant, _)| {
                    now.duration_since(*instant) >= TAKE_ITEM_ANIMATION_SPEED
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

pub fn player(
    runtime: &Runtime,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    gui_instances: &mut GuiInstances,
    context: &Context,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.player_menu]
            .as_str(),
    )
    .frame(default_frame())
    .resizable(false)
    .collapsible(false)
    .show(context, |ui| {
        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.player_inventory]
                .as_str(),
        );

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
            ScrollArea::vertical().show(ui, |ui| {
                for (item, amount) in inventory.iter().flat_map(|(id, amount)| {
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
                        MEDIUM_ITEM_ICON_SIZE,
                    );

                    take_item_animation(item, dst_rect, setup, loop_store, gui_instances);
                }
            });
        }

        if ui
            .button(
                setup.resource_man.translates.gui
                    [&setup.resource_man.registry.gui_ids.open_research]
                    .as_str(),
            )
            .clicked()
        {
            loop_store.switch_gui_state(GuiState::Research);
        }
    });
}
