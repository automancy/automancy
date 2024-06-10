use std::f64::consts::FRAC_PI_4;

use tokio::sync::oneshot;

use automancy_defs::glam::{dvec3, vec2, vec3, FloatExt};
use automancy_defs::id::Id;
use automancy_defs::math::{z_far, z_near, DMatrix4, Float, Matrix4};
use automancy_defs::rendering::InstanceData;
use automancy_defs::{colors, math};
use automancy_resources::format;
use automancy_resources::{
    data::{Data, DataMap},
    types::IconMode,
};
use yakui::{
    column, use_state,
    widgets::{Absolute, Layer},
    Alignment, Dim2, Pivot,
};

use crate::util::is_research_unlocked;
use crate::GameState;

use super::{
    centered_column, centered_row, hover_tip, interactive, label, scroll_horizontal,
    ui_game_object, RoundRect, LARGE_ICON_SIZE, MEDIUM_ICON_SIZE,
};

fn tile_hover_z_angle(elapsed: Float, hovered: bool) -> Float {
    fn angle(hovered: bool) -> Float {
        if hovered {
            0.75
        } else {
            0.25
        }
    }

    //TODO extract this
    let s = use_state(move || angle(hovered));

    let target = angle(hovered);

    let r = s.get();

    s.modify(|v| {
        let lerped = v.lerp(target, elapsed);

        lerped.clamp(v.min(target), v.max(target))
    });

    r
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
    game_data: &mut DataMap,
    selection_send: &mut Option<oneshot::Sender<Id>>,
    current_category: Option<Id>,
    size: Float,
) -> Option<(Id, bool)> {
    let projection = DMatrix4::perspective_lh(FRAC_PI_4, 1.0, z_near(), z_far())
        * math::view(dvec3(0.0, 0.0, 2.75));
    let projection = projection.as_mat4();

    let has_item = if let Some(category) = current_category {
        has_category_item(state, game_data, category)
    } else {
        true
    };

    let mut hovered = None;

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
                if !is_research_unlocked(research.id, &state.resource_man, game_data) {
                    continue;
                }
            } else {
                continue;
            }
        }

        let active = is_default_tile || has_item;

        let tile = state.resource_man.registry.tiles.get(id).unwrap();
        let model = state.resource_man.tile_model_or_missing(tile.model);

        let hover_anim_active = use_state(|| false);

        let rotate = Matrix4::from_rotation_x(tile_hover_z_angle(
            state.loop_store.elapsed.as_secs_f32() * 5.0,
            hover_anim_active.get(),
        ));

        let color_offset = if active {
            Default::default()
        } else {
            colors::INACTIVE.to_linear()
        };

        let response = interactive(|| {
            ui_game_object(
                InstanceData::default()
                    .with_model_matrix(rotate)
                    .with_light_pos(vec3(0.0, 1.0, 8.0), None)
                    .with_color_offset(color_offset),
                model,
                vec2(size, size),
                Some(projection),
            );
        });

        hover_anim_active.set(response.hovering);

        if response.hovering {
            hovered = Some((*id, active));
        }

        if active && response.clicked {
            if let Some(send) = selection_send.take() {
                send.send(*id).unwrap();
            }
        }
    }

    hovered
}

/// Creates the tile selection GUI.
pub fn tile_selections(
    state: &mut GameState,
    game_data: &mut DataMap,
    selection_send: oneshot::Sender<Id>,
) {
    let world_matrix = IconMode::Tile.world_matrix();
    let model_matrix = IconMode::Tile.model_matrix();

    let mut hovered_category = None;
    let mut hovered_tile = None;

    Absolute::new(Alignment::BOTTOM_CENTER, Pivot::BOTTOM_CENTER, Dim2::ZERO).show(|| {
        centered_column(|| {
            RoundRect::new(8.0, colors::BACKGROUND_1).show_children(|| {
                scroll_horizontal(
                    state
                        .gui
                        .as_ref()
                        .unwrap()
                        .yak
                        .layout_dom()
                        .viewport()
                        .size()
                        .x,
                    || {
                        centered_row(|| {
                            for id in &state.resource_man.ordered_categories {
                                let category = &state.resource_man.registry.categories[id];
                                let model = state.resource_man.tile_model_or_missing(category.icon);

                                let response = interactive(|| {
                                    ui_game_object(
                                        InstanceData::default()
                                            .with_model_matrix(model_matrix)
                                            .with_light_pos(vec3(0.0, 1.0, 8.0), None),
                                        model,
                                        vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                                        Some(world_matrix),
                                    );
                                });

                                if response.clicked {
                                    state.gui_state.tile_selection_category = Some(*id);
                                }

                                if response.hovering {
                                    hovered_category = Some(*id);
                                }
                            }
                        });
                    },
                );
            });

            RoundRect::new(8.0, colors::BACKGROUND_1).show_children(|| {
                scroll_horizontal(
                    state
                        .gui
                        .as_ref()
                        .unwrap()
                        .yak
                        .layout_dom()
                        .viewport()
                        .size()
                        .x,
                    || {
                        centered_row(|| {
                            hovered_tile = draw_tile_selection(
                                state,
                                game_data,
                                &mut Some(selection_send),
                                state.gui_state.tile_selection_category,
                                LARGE_ICON_SIZE,
                            );
                        });
                    },
                );
            });
        });
    });

    Layer::new().show(|| {
        if let Some(id) = hovered_category {
            hover_tip(|| {
                label(&state.resource_man.category_name(&id));
            });
        }

        if let Some((id, active)) = hovered_tile {
            hover_tip(|| {
                column(|| {
                    label(&state.resource_man.tile_name(&id));

                    if !active {
                        if let Some(item) = state
                            .gui_state
                            .tile_selection_category
                            .and_then(|id| state.resource_man.registry.categories[&id].item)
                        {
                            label(&format(
                                state
                                    .resource_man
                                    .gui_str(
                                        &state
                                            .resource_man
                                            .registry
                                            .gui_ids
                                            .lbl_cannot_place_missing_item,
                                    )
                                    .as_str(),
                                &[&state.resource_man.item_name(&item)],
                            ));
                        };
                    }
                });
            });
        }
    });
}
