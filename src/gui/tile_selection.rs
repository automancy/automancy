use std::collections::HashSet;
use std::f64::consts::FRAC_PI_4;

use egui::scroll_area::ScrollBarVisibility;
use egui::{vec2, CursorIcon, Frame, Margin, Response, ScrollArea, Sense, TopBottomPanel, Ui};
use tokio::sync::oneshot;

use automancy_defs::glam::{dvec3, vec3};
use automancy_defs::id::Id;
use automancy_defs::math::{z_far, z_near, DMatrix4, Float, Matrix4};
use automancy_defs::rendering::InstanceData;
use automancy_defs::{colors, math};
use automancy_resources::data::{Data, DataMap};
use automancy_resources::format;

use crate::gui::{GameEguiCallback, LARGE_ICON_SIZE, MEDIUM_ICON_SIZE};
use crate::GameState;

fn tile_hover_z_angle(ui: &Ui, response: &Response) -> Float {
    if response.hovered() {
        ui.ctx()
            .animate_value_with_time(ui.next_auto_id(), 0.75, 0.3)
    } else {
        ui.ctx()
            .animate_value_with_time(ui.next_auto_id(), 0.25, 0.3)
    }
}

fn has_category_item(state: &mut GameState, game_data: &mut DataMap, id: Id) -> bool {
    let category = &state.resource_man.registry.categories[&id];

    if let Some(item) = category.item {
        if let Some(Data::Inventory(inventory)) =
            game_data.get_mut(&state.resource_man.registry.data_ids.player_inventory)
        {
            inventory.get(item) > 0
        } else {
            false
        }
    } else {
        true
    }
}

/// Draws the tile selection.
fn draw_tile_selection(
    state: &mut GameState,
    ui: &mut Ui,
    game_data: &mut DataMap,
    selection_send: &mut Option<oneshot::Sender<Id>>,
    current_category: Option<Id>,
) {
    let size = ui.available_height();
    let projection = DMatrix4::perspective_lh(FRAC_PI_4, 1.0, z_near(), z_far())
        * math::view(dvec3(0.0, 0.0, 2.75));
    let projection = projection.as_mat4();

    let has_item = if let Some(category) = current_category {
        has_category_item(state, game_data, category)
    } else {
        true
    };

    for id in &state.resource_man.ordered_tiles {
        if let Some(Data::Id(category)) = state.resource_man.registry.tiles[id]
            .data
            .get(&state.resource_man.registry.data_ids.category)
        {
            if Some(*category) != current_category {
                continue;
            }
        }

        let is_default_tile = match state.resource_man.registry.tiles[id]
            .data
            .get(&state.resource_man.registry.data_ids.default_tile)
        {
            Some(Data::Bool(v)) => *v,
            _ => false,
        };

        if !is_default_tile {
            if let Some(research) = state.resource_man.get_research_by_unlock(*id) {
                if let Data::SetId(unlocked) = game_data
                    .entry(state.resource_man.registry.data_ids.unlocked_researches)
                    .or_insert_with(|| Data::SetId(HashSet::new()))
                {
                    if !unlocked.contains(&research.id) {
                        continue;
                    }
                } else {
                    game_data.remove(&state.resource_man.registry.data_ids.unlocked_researches);
                }
            }
        }

        let tile = state.resource_man.registry.tiles.get(id).unwrap();
        let model = state.resource_man.get_model(tile.model);

        let (ui_id, rect) = ui.allocate_space(vec2(size, size));

        let response = ui
            .interact(rect, ui_id, Sense::click())
            .on_hover_text(state.resource_man.tile_name(id))
            .on_hover_cursor(CursorIcon::Grab);

        let response = if !(is_default_tile || has_item) {
            if let Some(item) =
                current_category.and_then(|id| state.resource_man.registry.categories[&id].item)
            {
                response
                    .on_hover_text(format(
                        state.resource_man.translates.gui[&state
                            .resource_man
                            .registry
                            .gui_ids
                            .lbl_cannot_place_missing_item]
                            .as_str(),
                        &[state.resource_man.item_name(&item)],
                    ))
                    .on_hover_cursor(CursorIcon::NotAllowed)
            } else {
                response
            }
        } else {
            response
        };

        if response.clicked() {
            if let Some(send) = selection_send.take() {
                send.send(*id).unwrap();
            }
        }

        let rotate = Matrix4::from_rotation_x(tile_hover_z_angle(ui, &response));

        let color_offset = if is_default_tile || has_item {
            Default::default()
        } else {
            colors::INACTIVE.to_array()
        };

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            GameEguiCallback::new(
                InstanceData::default()
                    .with_model_matrix(rotate)
                    .with_world_matrix(projection)
                    .with_light_pos(vec3(0.0, 4.0, 14.0), None)
                    .with_color_offset(color_offset),
                model,
                rect,
                ui.ctx().screen_rect(),
            ),
        ));
    }
}

/// Creates the tile selection GUI.
pub fn tile_selections(
    state: &mut GameState,
    game_data: &mut DataMap,
    selection_send: oneshot::Sender<Id>,
) {
    let projection = DMatrix4::perspective_lh(FRAC_PI_4, 1.0, z_near(), z_far())
        * math::view(dvec3(0.0, 0.0, 2.75));
    let projection = projection.as_mat4();

    TopBottomPanel::bottom("tile_selections")
        .show_separator_line(false)
        .resizable(false)
        .frame(Frame::window(&state.gui.context.clone().style()).outer_margin(Margin::same(10.0)))
        .show(&state.gui.context.clone(), |ui| {
            ScrollArea::horizontal()
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_height(LARGE_ICON_SIZE);

                        draw_tile_selection(
                            state,
                            ui,
                            game_data,
                            &mut Some(selection_send),
                            state.gui_state.tile_selection_category,
                        );
                    });
                });
        });

    TopBottomPanel::bottom("category_selections")
        .show_separator_line(false)
        .resizable(false)
        .frame(
            Frame::window(&state.gui.context.clone().style())
                .outer_margin(Margin::symmetric(40.0, 0.0)),
        )
        .show(&state.gui.context.clone(), |ui| {
            ScrollArea::horizontal()
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_height(MEDIUM_ICON_SIZE);

                        for id in &state.resource_man.ordered_categories {
                            let category = &state.resource_man.registry.categories[id];
                            let model = state.resource_man.get_model(category.icon);
                            let size = ui.available_height();

                            let (ui_id, rect) = ui.allocate_space(vec2(size, size));

                            let response = ui
                                .interact(rect, ui_id, Sense::click())
                                .on_hover_text(state.resource_man.category_name(id))
                                .on_hover_cursor(CursorIcon::Grab);
                            if response.clicked() {
                                state.gui_state.tile_selection_category = Some(*id)
                            }

                            let rotate =
                                Matrix4::from_rotation_x(tile_hover_z_angle(ui, &response));

                            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                                rect,
                                GameEguiCallback::new(
                                    InstanceData::default()
                                        .with_model_matrix(rotate)
                                        .with_world_matrix(projection)
                                        .with_light_pos(vec3(0.0, 4.0, 14.0), None),
                                    model,
                                    rect,
                                    ui.ctx().screen_rect(),
                                ),
                            ));
                        }
                    });
                });
        });
}
