use egui::scroll_area::ScrollBarVisibility;
use egui::{vec2, ScrollArea, Sense, Window};

use automancy_defs::glam::vec3;
use automancy_defs::graph::visit::Topo;
use automancy_defs::math::Float;
use automancy_defs::rendering::InstanceData;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::types::IconMode;

use crate::gui::item::draw_item;
use crate::gui::{take_item_animation, GameEguiCallback, MEDIUM_ICON_SIZE};
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
                        });
                    }
                });
            });
        });
    });
}
