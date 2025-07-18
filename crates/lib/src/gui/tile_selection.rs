use automancy_defs::{
    colors,
    id::{Id, ModelId, TileId},
    math::{Float, Matrix4, vec2},
    rendering::InstanceData,
};
use automancy_resources::{
    data::{Data, DataMap},
    types::IconMode,
};
use automancy_system::util::{is_research_unlocked, should_category_show};
use automancy_ui::{
    LARGE_ICON_SIZE, MEDIUM_ICON_SIZE, RoundRect, UiGameObjectType, center_col, col, hover_tip,
    interactive, label, row, scroll_horizontal_bar_alignment, ui_game_object,
};
use interpolator::Formattable;
use tokio::sync::oneshot;
use yakui::{Alignment, Dim2, Pivot, Vec2, reflow, use_state, widgets::Layer};

use crate::GameState;

fn tile_hover_z_angle(elapsed: Float, hovered: bool) -> Float {
    fn angle(hovered: bool) -> Float {
        if hovered { 0.5 } else { 0.0 }
    }

    //TODO extract this
    let s = use_state(move || angle(hovered));

    let target = angle(hovered);

    let r = s.get();

    s.modify(|v| {
        let lerped = v.lerp(target, elapsed);

        lerped.clamp(v.min(target + 0.01), v.max(target - 0.01))
    });

    r
}

fn has_category_item(state: &mut GameState, game_data: &mut DataMap, id: Id) -> bool {
    let category = state.resource_man.registry.categories[&id];

    if let Some(item) = category.item {
        if let Some(Data::Inventory(inventory)) =
            game_data.get_mut(state.resource_man.registry.data_ids.player_inventory)
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
    selection_send: &mut Option<oneshot::Sender<TileId>>,
    current_category: Option<Id>,
    size: Float,
) -> Option<(TileId, bool)> {
    let world_matrix = IconMode::Tile.world_matrix();

    let has_item = if let Some(category) = current_category {
        has_category_item(state, game_data, category)
    } else {
        true
    };

    let mut hovered = None;

    for id in &state.resource_man.ordered_tiles {
        if let Some(category) = state.resource_man.registry.tiles[id].category {
            if Some(category) != current_category {
                continue;
            }
        }

        let is_default_tile = match state.resource_man.registry.tiles[id]
            .data
            .get(state.resource_man.registry.data_ids.default_tile)
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
                InstanceData::default().with_color_offset(color_offset),
                UiGameObjectType::Tile(*id, DataMap::default()),
                vec2(size, size),
                Some(rotate),
                Some(world_matrix),
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
    selection_send: oneshot::Sender<TileId>,
) {
    let world_matrix = IconMode::Tile.world_matrix();
    let model_matrix = IconMode::Tile.model_matrix();

    let mut hovered_category = None;
    let mut hovered_tile = None;

    Layer::new().show(|| {
        reflow(
            Alignment::BOTTOM_CENTER,
            Pivot::BOTTOM_CENTER,
            Dim2::ZERO,
            || {
                center_col(|| {
                    RoundRect::new(8.0, colors::BACKGROUND_1).show_children(|| {
                        scroll_horizontal_bar_alignment(Vec2::ZERO, Vec2::INFINITY, None, || {
                            row(|| {
                                for id in &state.resource_man.ordered_categories {
                                    if !should_category_show(*id, &state.resource_man, game_data) {
                                        continue;
                                    }

                                    let category = state.resource_man.registry.categories[id];

                                    let ty = match category.icon_mode {
                                        IconMode::Item => UiGameObjectType::Model(
                                            state
                                                .resource_man
                                                .model_or_missing_item(&ModelId(category.icon)),
                                        ),
                                        IconMode::Tile => UiGameObjectType::Tile(
                                            TileId(category.icon),
                                            DataMap::default(),
                                        ),
                                    };

                                    let response = interactive(|| {
                                        ui_game_object(
                                            InstanceData::default(),
                                            ty,
                                            vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                                            Some(model_matrix),
                                            Some(world_matrix),
                                        );
                                    });

                                    if response.clicked {
                                        state.ui_state.tile_selection_category = Some(*id);
                                    }

                                    if response.hovering {
                                        hovered_category = Some(*id);
                                    }
                                }
                            });
                        });
                    });

                    RoundRect::new(8.0, colors::BACKGROUND_1).show_children(|| {
                        scroll_horizontal_bar_alignment(Vec2::ZERO, Vec2::INFINITY, None, || {
                            row(|| {
                                hovered_tile = draw_tile_selection(
                                    state,
                                    game_data,
                                    &mut Some(selection_send),
                                    state.ui_state.tile_selection_category,
                                    LARGE_ICON_SIZE,
                                );
                            });
                        });
                    });
                });
            },
        );
    });

    Layer::new().show(|| {
        if let Some(id) = hovered_category {
            hover_tip(|| {
                label(&state.resource_man.category_name(id));
            });
        }

        if let Some((id, active)) = hovered_tile {
            hover_tip(|| {
                col(|| {
                    label(&state.resource_man.tile_name(id));

                    if !active {
                        if let Some(item) = state
                            .ui_state
                            .tile_selection_category
                            .and_then(|id| state.resource_man.registry.categories[&id].item)
                        {
                            label(
                                &state.resource_man.gui_fmt(
                                    state
                                        .resource_man
                                        .registry
                                        .gui_ids
                                        .lbl_cannot_place_missing_item,
                                    [(
                                        "item_name",
                                        Formattable::display(&state.resource_man.item_name(item)),
                                    )],
                                ),
                            );
                        };
                    }
                });
            });
        }
    });
}
