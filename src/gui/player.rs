use std::time::Instant;

use egui::{Context, Rect, ScrollArea, Ui, Window};
use hashbrown::HashMap;

use automancy_defs::glam::dvec3;
use automancy_defs::math;
use automancy_defs::rendering::InstanceData;
use automancy_resources::data::item::Item;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};

use crate::event::EventLoopStorage;
use crate::game::TAKE_ITEM_ANIMATION_SPEED;
use crate::gui::item::draw_item;
use crate::gui::{GameEguiCallback, Screen, MEDIUM_ICON_SIZE};
use crate::setup::GameSetup;

fn take_item_animation(
    ui: &mut Ui,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    item: Item,
    dst_rect: Rect,
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
            let rect = src_rect.lerp_towards(&dst_rect, d);

            ui.ctx()
                .layer_painter(ui.layer_id())
                .add(egui_wgpu::Callback::new_paint_callback(
                    rect,
                    GameEguiCallback::new(
                        InstanceData::default()
                            .with_world_matrix(math::view(dvec3(0.0, 0.0, 1.0)).as_mat4()),
                        setup.resource_man.get_item_model(item),
                        rect,
                        ui.ctx().screen_rect(),
                    ),
                ));
        }
    }
}

pub fn player(
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    context: &Context,
    game_data: &DataMap,
) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.player_menu]
            .as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .show(context, |ui| {
        ui.label(
            setup.resource_man.translates.gui
                [&setup.resource_man.registry.gui_ids.player_inventory]
                .as_str(),
        );

        if let Some(Data::Inventory(inventory)) =
            game_data.get(&setup.resource_man.registry.data_ids.player_inventory)
        {
            ScrollArea::vertical().show(ui, |ui| {
                for (id, amount) in inventory.iter() {
                    let item = setup.resource_man.registry.items.get(id).unwrap();

                    let (dst_rect, _) = draw_item(
                        ui,
                        &setup.resource_man,
                        None,
                        ItemStack {
                            item: *item,
                            amount: *amount,
                        },
                        MEDIUM_ICON_SIZE,
                        true,
                    );

                    take_item_animation(ui, setup, loop_store, *item, dst_rect);
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
            loop_store.gui_state.switch_screen(Screen::Research);
        }
    });
}
