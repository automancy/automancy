use crate::*;

#[cfg_attr(feature = "profile", profiling::function)]
fn has_category_item(game_state: &mut AutomancyGameState, map_data: &DataMap, id: CategoryId) -> bool {
    let category = game_state.resource_man.registry.category_defs[&id];

    if !category.item.is_none() {
        map_data
            .inventory_then(game_state.resource_man.registry.data_ids.player_inventory, |inventory| {
                inventory.get(category.item) > 0
            })
            .unwrap_or(false)
    } else {
        true
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn draw_tile_selection_list(ctx: &mut UiContext, current_category: Option<CategoryId>, size: f32) -> TileSelectionResult {
    let mut result = TileSelectionResult::default();

    let has_category_item = if let Some(category) = current_category {
        has_category_item(ctx.game_state, &ctx.game_data.map_data, category)
    } else {
        true
    };

    for &id in &ctx.game_state.resource_man.ordered_tiles {
        let tile_def = &ctx.game_state.resource_man.registry.tile_defs[&id];

        if !tile_def.category.is_none() && Some(tile_def.category) != current_category {
            continue;
        }

        let is_default_tile = tile_def.id.is_none()
            || tile_def
                .data
                .bool(ctx.game_state.resource_man.registry.data_ids.default_tile)
                .copied()
                .unwrap_or(false);

        if !is_default_tile {
            if let Some(research) = ctx.game_state.resource_man.get_research_by_unlock(id) {
                if !ctx
                    .game_state
                    .resource_man
                    .is_research_unlocked(research.id, &ctx.game_data.map_data)
                {
                    continue;
                }
            } else {
                continue;
            }
        }
        let is_active = is_default_tile || has_category_item;

        let hover_anim_active = use_state(|| false);

        let color_offset = if is_active {
            colors::TRANSPARENT
        } else {
            colors::BACKGROUND_INACTIVE
        };

        let rotate = math::Matrix4::rotation_x(
            AnimatedValue::new(&[(0.0, 0.0), (1.0, -0.4)])
                .ease_duration(Duration::from_millis(500))
                .ease_function(easings::EaseOutCubic)
                .show(if hover_anim_active.get() { 1.0 } else { 0.0 }),
        );
        let model_matrix = rotate;
        let world_matrix = GenericModel::Tile(id).view_matrix();

        let response = interactive(|| {
            GameModel::new_with_matrix(GenericModel::Tile(id), Vec2::splat(size), (model_matrix, world_matrix))
                .color(color_offset.packed)
                .show();
        });

        hover_anim_active.set(response.hovering);

        if response.hovering {
            result.hovering_tile = Some(id);
            result.is_active = is_active;

            if is_active && response.clicked {
                result.clicked = true;
            }
        }
    }

    result
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn draw_category_selection_list(ctx: &mut UiContext) -> Option<CategoryId> {
    let mut hovered_id = None;

    for &id in &ctx.game_state.resource_man.ordered_categories {
        if !ctx.game_state.resource_man.should_category_show(id, &ctx.game_data.map_data) {
            continue;
        }

        let category = ctx.game_state.resource_man.registry.category_defs[&id];

        let response = interactive(|| {
            GameModel::new(category.icon, Vec2::splat(sizing::MEDIUM_ICON_SIZE)).show();
        });

        if response.clicked {
            ctx.gui.state.tile_selection_category = Some(id);
        }

        if response.hovering {
            hovered_id = Some(id);
        }
    }

    hovered_id
}

#[derive(Debug, Default)]
pub struct TileSelectionResult {
    pub hovering_category: Option<CategoryId>,
    pub hovering_tile: Option<TileId>,
    pub is_active: bool,
    pub clicked: bool,
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn tile_selection_widget(ctx: &mut UiContext) -> TileSelectionResult {
    let mut selection_result = TileSelectionResult::default();
    let mut hovering_category = None;

    Layer::new().show(|| {
        align(Alignment::BOTTOM_CENTER, || {
            list_col().cross_axis_alignment(CrossAxisAlignment::Center).show(|| {
                RoundRect::new(sizing::ROUNDED_LARGE)
                    .color(colors::BACKGROUND_1.yak())
                    .show_children(|| {
                        Scrollable::horizontal().show(|| {
                            row(|| {
                                hovering_category = draw_category_selection_list(ctx);
                            });
                        });
                    });

                RoundRect::new(sizing::ROUNDED_LARGE)
                    .color(colors::BACKGROUND_1.yak())
                    .show_children(|| {
                        Scrollable::horizontal().show(|| {
                            row(|| {
                                selection_result =
                                    draw_tile_selection_list(ctx, ctx.gui.state.tile_selection_category, sizing::LARGE_ICON_SIZE);
                            });
                        });
                    });

                selection_result.hovering_category = hovering_category;
            });
        });

        if let Some(id) = selection_result.hovering_category {
            hover_tip(|| {
                Text::normal(ctx.game_state.resource_man.category_name(id).to_string()).show();
            });
        }

        if let Some(id) = selection_result.hovering_tile {
            hover_tip(|| {
                col(|| {
                    Text::normal(ctx.game_state.resource_man.tile_name(id).to_string()).show();

                    if !selection_result.is_active
                        && let Some(item) = ctx
                            .gui
                            .state
                            .tile_selection_category
                            .map(|id| ctx.game_state.resource_man.registry.category_defs[&id].item)
                    {
                        Text::normal(gui_str!(
                            ctx.game_state,
                            tile_missing_item_label,
                            [("item_name", Formattable::display(&ctx.game_state.resource_man.item_name(item)),)]
                        ))
                        .show();
                    };
                });
            });
        }
    });

    selection_result
}
