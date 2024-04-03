use egui::scroll_area::ScrollBarVisibility;
use egui::{vec2, Button, ScrollArea, Sense, Window};

use automancy_defs::glam::vec3;
use automancy_defs::graph::visit::Topo;
use automancy_defs::math::Float;
use automancy_defs::rendering::InstanceData;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::types::IconMode;

use crate::gui::item::draw_item;
use crate::gui::{take_item_animation, GameEguiCallback, MEDIUM_ICON_SIZE, SMALL_ICON_SIZE};
use crate::util::is_research_unlocked;
use crate::GameState;

pub fn player(state: &mut GameState, game_data: &mut DataMap) {
    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.player_menu]
            .as_str(),
    )
    .id("player_menu".into())
    .collapsible(false)
    .auto_sized()
    .show(&state.gui.context.clone(), |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.heading(
                    state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.player_inventory_title]
                        .as_str(),
                );

                if let Some(Data::Inventory(inventory)) =
                    game_data.get(&state.resource_man.registry.data_ids.player_inventory)
                {
                    ScrollArea::vertical()
                        .id_source("player_inventory")
                        .drag_to_scroll(true)
                        .show(ui, |ui| {
                            for (id, amount) in inventory.iter() {
                                if *amount != 0 {
                                    if let Some(item) = state.resource_man.registry.items.get(id) {
                                        let (dst_rect, _) = draw_item(
                                            &state.resource_man,
                                            ui,
                                            None,
                                            ItemStack {
                                                item: *item,
                                                amount: *amount,
                                            },
                                            MEDIUM_ICON_SIZE,
                                            true,
                                        );

                                        take_item_animation(state, ui, *item, dst_rect);
                                    }
                                }
                            }
                        });
                }
            });

            ui.add_space(30.0);

            ui.vertical(|ui| {
                ui.heading(
                    state.resource_man.translates.gui
                        [&state.resource_man.registry.gui_ids.research_menu_title]
                        .as_str(),
                );

                ui.horizontal_top(|ui| {
                    ui.group(|ui| {
                        const WIDTH: Float = 200.0;

                        ScrollArea::vertical()
                            .id_source("research_list")
                            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                            .drag_to_scroll(true)
                            .auto_shrink(false)
                            .max_height(160.0)
                            .max_width(WIDTH)
                            .show(ui, |ui| {
                                let mut visitor =
                                    Topo::new(&state.resource_man.registry.researches);

                                ui.vertical(|ui| {
                                    while let Some(idx) =
                                        visitor.next(&state.resource_man.registry.researches)
                                    {
                                        let research = &state.resource_man.registry.researches[idx];
                                        let icon = match research.icon_mode {
                                            IconMode::Item => {
                                                state.resource_man.get_item_model(research.icon)
                                            }
                                            IconMode::Tile => {
                                                state.resource_man.get_model(research.icon)
                                            }
                                        };

                                        if let Some(prev) = research.depends_on {
                                            if !is_research_unlocked(
                                                prev,
                                                &state.resource_man,
                                                game_data,
                                            ) {
                                                continue;
                                            }
                                        }

                                        if ui
                                            .scope(|ui| {
                                                ui.set_width(WIDTH);

                                                ui.horizontal(|ui| {
                                                    let (rect, _icon_response) = ui
                                                        .allocate_exact_size(
                                                            vec2(
                                                                MEDIUM_ICON_SIZE,
                                                                MEDIUM_ICON_SIZE,
                                                            ),
                                                            Sense::click(),
                                                        );

                                                    ui.painter().add(
                                                        egui_wgpu::Callback::new_paint_callback(
                                                            rect,
                                                            GameEguiCallback::new(
                                                                InstanceData::default()
                                                                    .with_model_matrix(
                                                                        research
                                                                            .icon_mode
                                                                            .model_matrix(),
                                                                    )
                                                                    .with_world_matrix(
                                                                        research
                                                                            .icon_mode
                                                                            .world_matrix(),
                                                                    )
                                                                    .with_light_pos(
                                                                        vec3(0.0, 4.0, 14.0),
                                                                        None,
                                                                    ),
                                                                icon,
                                                                rect,
                                                                ui.ctx().screen_rect(),
                                                            ),
                                                        ),
                                                    );

                                                    ui.label(
                                                        state
                                                            .resource_man
                                                            .research_str(&research.name),
                                                    );
                                                });
                                            })
                                            .response
                                            .interact(Sense::click())
                                            .clicked()
                                        {
                                            state.gui_state.selected_research = Some(research.id);
                                        };
                                    }
                                });
                            });
                    });

                    if let Some(research) = state
                        .gui_state
                        .selected_research
                        .and_then(|id| state.resource_man.get_research(id))
                    {
                        ui.vertical(|ui| {
                            ui.heading(state.resource_man.research_str(&research.name));
                            ui.label(state.resource_man.research_str(&research.description));

                            ScrollArea::vertical()
                                .id_source("research_item_list")
                                .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                                .drag_to_scroll(true)
                                .auto_shrink(false)
                                .max_height(120.0)
                                .max_width(160.0)
                                .show(ui, |ui| {
                                    if let Some(stacks) = &research.required_items {
                                        for stack in stacks {
                                            draw_item(
                                                &state.resource_man,
                                                ui,
                                                None,
                                                *stack,
                                                SMALL_ICON_SIZE,
                                                true,
                                            );
                                        }
                                    }
                                });

                            let mut already_filled = false;
                            if let Some(Data::SetId(items_filled)) = game_data
                                .get(&state.resource_man.registry.data_ids.research_items_filled)
                            {
                                already_filled = items_filled.contains(&research.id)
                            }

                            if let Some(stacks) = &research.required_items {
                                if ui
                                    .add_enabled(
                                        !already_filled,
                                        Button::new(
                                            state.resource_man.translates.gui[&state
                                                .resource_man
                                                .registry
                                                .gui_ids
                                                .research_submit_items]
                                                .as_str(),
                                        ),
                                    )
                                    .clicked()
                                    && !already_filled
                                {
                                    let mut can_take = false;
                                    if let Some(Data::Inventory(inventory)) = game_data.get_mut(
                                        &state.resource_man.registry.data_ids.player_inventory,
                                    ) {
                                        can_take = stacks.iter().all(|v| inventory.contains(*v))
                                    }

                                    if can_take {
                                        if let Some(Data::Inventory(inventory)) = game_data.get_mut(
                                            &state.resource_man.registry.data_ids.player_inventory,
                                        ) {
                                            for stack in stacks {
                                                inventory.take(stack.item.id, stack.amount);
                                            }
                                        }

                                        if let Some(Data::SetId(items_filled)) = game_data.get_mut(
                                            &state
                                                .resource_man
                                                .registry
                                                .data_ids
                                                .research_items_filled,
                                        ) {
                                            items_filled.insert(research.id);
                                        }
                                    }
                                }
                            }
                        });

                        let mut a = false;
                        let mut b = false;
                        let mut ab = false;
                        {
                            if !game_data.contains_key(
                                &state.resource_man.registry.data_ids.research_items_filled,
                            ) {
                                game_data.insert(
                                    state.resource_man.registry.data_ids.research_items_filled,
                                    Data::SetId(Default::default()),
                                );
                            }

                            if !game_data.contains_key(
                                &state
                                    .resource_man
                                    .registry
                                    .data_ids
                                    .research_puzzle_completed,
                            ) {
                                game_data.insert(
                                    state
                                        .resource_man
                                        .registry
                                        .data_ids
                                        .research_puzzle_completed,
                                    Data::SetId(Default::default()),
                                );
                            }

                            if let Some((
                                Data::SetId(filled_items),
                                Data::SetId(completed_puzzles),
                            )) = game_data
                                .get(&state.resource_man.registry.data_ids.research_items_filled)
                                .zip(
                                    game_data.get(
                                        &state
                                            .resource_man
                                            .registry
                                            .data_ids
                                            .research_puzzle_completed,
                                    ),
                                )
                            {
                                a = research.attached_puzzle.is_none()
                                    && filled_items.contains(&research.id);

                                b = research.required_items.is_none()
                                    & &completed_puzzles.contains(&research.id);

                                ab = filled_items.contains(&research.id)
                                    & &completed_puzzles.contains(&research.id);
                            }
                        }

                        if a || b || ab {
                            if let Some(Data::SetId(set)) = game_data.get_mut(
                                &state.resource_man.registry.data_ids.research_items_filled,
                            ) {
                                set.remove(&research.id);
                            }
                            if let Some(Data::SetId(set)) = game_data.get_mut(
                                &state
                                    .resource_man
                                    .registry
                                    .data_ids
                                    .research_puzzle_completed,
                            ) {
                                set.remove(&research.id);
                            }

                            if let Data::SetId(set) = game_data
                                .entry(state.resource_man.registry.data_ids.unlocked_researches)
                                .or_insert_with(|| Data::SetId(Default::default()))
                            {
                                set.insert(research.id);
                            }
                        }
                    }
                });
            });
        });
    });
}
