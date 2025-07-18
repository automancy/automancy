use std::mem;

use automancy_defs::{
    colors,
    colors::BACKGROUND_3,
    coord::TileCoord,
    id::{Id, ModelId},
    math::{Vec2, vec2},
    rendering::InstanceData,
    stack::ItemStack,
};
use automancy_resources::{
    data::{Data, DataMap},
    rhai_call_options, rhai_log_err,
    types::IconMode,
};
use automancy_system::{input::ActionType, util::is_research_unlocked};
use automancy_ui::{
    DIVIER_HEIGHT, DIVIER_THICKNESS, MEDIUM_ICON_SIZE, PADDING_MEDIUM, PositionRecord, RoundRect,
    SMALL_ICON_SIZE, TINY_ICON_SIZE, UiGameObjectType, button, centered_horizontal, col, group,
    heading, inactive_button, interactive, label, list_row, movable, row, scroll_horizontal,
    scroll_horizontal_bar_alignment, scroll_vertical, scroll_vertical_bar_alignment,
    ui_game_object, window_box,
};
use hexx::{HexLayout, HexOrientation};
use petgraph::visit::Topo;
use rhai::{Array, Dynamic, Scope};
use yakui::{
    Alignment, Constraints, Dim2, Pivot, Rect, constrained, divider, reflow,
    widgets::{Layer, Pad},
};

use super::{item::draw_item, util::take_item_animation};
use crate::GameState;

const PUZZLE_HEX_GRID_LAYOUT: HexLayout = HexLayout {
    orientation: HexOrientation::Pointy,
    origin: vec2(TINY_ICON_SIZE, 0.0),
    scale: vec2(TINY_ICON_SIZE, -TINY_ICON_SIZE),
};

fn player_inventory(state: &mut GameState, game_data: &mut DataMap) {
    heading(
        &state
            .resource_man
            .gui_str(state.resource_man.registry.gui_ids.player_inventory_title),
    );

    let Some(Data::Inventory(inventory)) =
        game_data.get(state.resource_man.registry.data_ids.player_inventory)
    else {
        return;
    };

    scroll_vertical(Vec2::ZERO, Vec2::new(f32::INFINITY, 200.0), || {
        group(|| {
            col(|| {
                for (id, amount) in inventory.iter() {
                    let amount = *amount;

                    if amount != 0 {
                        let pos = PositionRecord::new()
                            .show(|| {
                                draw_item(
                                    &state.resource_man,
                                    || {},
                                    ItemStack { id: *id, amount },
                                    MEDIUM_ICON_SIZE,
                                    true,
                                );
                            })
                            .into_inner();

                        if let Some(pos) = pos {
                            take_item_animation(
                                state,
                                *id,
                                Rect::from_pos_size(
                                    pos,
                                    Vec2::new(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                                ),
                            );
                        }
                    }
                }
            });
        });
    });
}

fn research_selection(state: &mut GameState, game_data: &mut DataMap) {
    heading(
        &state
            .resource_man
            .gui_str(state.resource_man.registry.gui_ids.research_menu_title),
    );

    let mut visitor = Topo::new(&state.resource_man.registry.researches);

    scroll_vertical(Vec2::ZERO, Vec2::new(f32::INFINITY, 200.0), || {
        group(|| {
            col(|| {
                while let Some(idx) = visitor.next(&state.resource_man.registry.researches) {
                    let research = &state.resource_man.registry.researches[idx];
                    let icon = match research.icon_mode {
                        IconMode::Tile => state.resource_man.model_or_missing_tile(&research.icon),
                        IconMode::Item => state.resource_man.model_or_missing_item(&research.icon),
                    };

                    if let Some(prev) = research.depends_on {
                        if !is_research_unlocked(prev, &state.resource_man, game_data) {
                            continue;
                        }
                    }

                    let interact = interactive(|| {
                        centered_horizontal(|| {
                            ui_game_object(
                                InstanceData::default(),
                                UiGameObjectType::Model(icon),
                                vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                                Some(research.icon_mode.model_matrix()),
                                Some(research.icon_mode.world_matrix()),
                            );

                            label(&state.resource_man.research_str(research.name));
                        });
                    });

                    if interact.clicked {
                        state.ui_state.selected_research = Some(research.id);
                        state.ui_state.selected_research_puzzle_tile = None;
                        state.ui_state.research_puzzle_selections = None;
                        state.puzzle_state = None; // TODO have a better save system for this
                        state.ui_state.force_show_puzzle = false;
                    };
                }
            });
        });
    });
}

fn current_research(state: &mut GameState, game_data: &mut DataMap) {
    let Some(research) = state
        .ui_state
        .selected_research
        .and_then(|id| state.resource_man.get_research(id))
    else {
        return;
    };

    heading(&state.resource_man.research_str(research.name));

    constrained(Constraints::loose(Vec2::new(500.0, f32::INFINITY)), || {
        label(&state.resource_man.research_str(research.description));
    });

    if !game_data.contains_id(
        state.resource_man.registry.data_ids.unlocked_researches,
        research.id,
    ) {
        let already_filled = game_data.contains_id(
            state.resource_man.registry.data_ids.research_items_filled,
            research.id,
        );

        scroll_vertical_bar_alignment(Vec2::ZERO, Vec2::new(240.0, 200.0), None, || {
            col(|| {
                if let Some(stacks) = &research.required_items {
                    for stack in stacks {
                        draw_item(&state.resource_man, || {}, *stack, SMALL_ICON_SIZE, true);
                    }
                }
            });
        });

        if let Some(stacks) = &research.required_items {
            let submit_text = &state
                .resource_man
                .gui_str(state.resource_man.registry.gui_ids.research_submit_items);

            let submit_button = if !already_filled {
                button(submit_text)
            } else {
                inactive_button(submit_text)
            };

            if !already_filled
                && submit_button.clicked
                && stacks.iter().all(|v| {
                    game_data
                        .contains_stack(state.resource_man.registry.data_ids.player_inventory, *v)
                })
            {
                if let Some(Data::Inventory(inventory)) =
                    game_data.get_mut(state.resource_man.registry.data_ids.player_inventory)
                {
                    for stack in stacks {
                        inventory.take(stack.id, stack.amount);
                    }
                }

                if let Some(Data::SetId(items_filled)) =
                    game_data.get_mut(state.resource_man.registry.data_ids.research_items_filled)
                {
                    items_filled.insert(research.id);
                }
            }
        }
    }
}

fn research_board_tiles(
    state: &mut GameState,
    tiles: impl IntoIterator<Item = (TileCoord, Id)>,
) -> bool {
    let interact = interactive(|| {
        col(|| {
            for (coord, id) in tiles.into_iter() {
                let pos = PUZZLE_HEX_GRID_LAYOUT.hex_to_world_pos(*coord);

                reflow(
                    Alignment::TOP_LEFT,
                    Pivot::TOP_LEFT,
                    Dim2::pixels(pos.x, pos.y),
                    || {
                        ui_game_object(
                            InstanceData::default(),
                            UiGameObjectType::Model(
                                state.resource_man.model_or_puzzle_space(&ModelId(id)),
                            ),
                            PUZZLE_HEX_GRID_LAYOUT.hex_size * 2.0,
                            None,
                            Some(IconMode::Item.world_matrix()),
                        );
                    },
                );
            }
        });
    });
    interact.clicked
}

fn research_puzzle(state: &mut GameState, game_data: &mut DataMap) -> Option<Vec2> {
    let research = state
        .ui_state
        .selected_research
        .and_then(|id| state.resource_man.get_research(id))?;

    let completed = game_data.contains_id(
        state
            .resource_man
            .registry
            .data_ids
            .research_puzzle_completed,
        research.id,
    );

    if !state.ui_state.force_show_puzzle {
        if research.required_items.is_some()
            && !game_data.contains_id(
                state.resource_man.registry.data_ids.research_items_filled,
                research.id,
            )
        {
            return None;
        }

        if completed {
            return None;
        }
    }

    let mut board_pos = None;
    if let Some(((ast, metadata), setup)) = state
        .ui_state
        .selected_research
        .and_then(|id| state.resource_man.get_research(id))
        .and_then(|research| research.attached_puzzle.as_ref())
        .and_then(|(id, setup)| state.resource_man.functions.get(id).zip(Some(setup)))
    {
        let puzzle_state = state.puzzle_state.get_or_insert_with(|| {
            let mut rhai_state = Dynamic::from(DataMap::default());

            let result = state.resource_man.engine.call_fn_with_options::<()>(
                rhai_call_options(&mut rhai_state),
                &mut Scope::default(),
                ast,
                "pre_setup",
                (Dynamic::from(setup.clone()),),
            );

            if let Err(err) = result {
                rhai_log_err("pre_setup", &metadata.str_id, &err, None)
            }

            (rhai_state.take().cast::<DataMap>(), true)
        });

        if puzzle_state.1 {
            let mut rhai_state = Dynamic::from(mem::take(&mut puzzle_state.0));

            let result = state.resource_man.engine.call_fn_with_options::<bool>(
                rhai_call_options(&mut rhai_state),
                &mut Scope::new(),
                ast,
                "evaluate",
                (Dynamic::from(setup.clone()),),
            );

            *puzzle_state = (rhai_state.take().cast::<DataMap>(), false);

            match result {
                Ok(result) => {
                    if result {
                        if let Data::SetId(set) = game_data
                            .entry(
                                state
                                    .resource_man
                                    .registry
                                    .data_ids
                                    .research_puzzle_completed,
                            )
                            .or_insert_with(|| Data::SetId(Default::default()))
                        {
                            set.insert(research.id);
                        }
                    }
                }
                Err(err) => rhai_log_err("evaluate", &metadata.str_id, &err, None),
            }
        }

        if let Some(selected) = state.ui_state.selected_research_puzzle_tile {
            let mut rhai_state = Dynamic::from(mem::take(&mut puzzle_state.0));

            let result = state.resource_man.engine.call_fn_with_options::<Array>(
                rhai_call_options(&mut rhai_state),
                &mut Scope::new(),
                ast,
                "selection_at_coord",
                (Dynamic::from(setup.clone()), selected),
            );

            state.puzzle_state = Some((rhai_state.take().cast::<DataMap>(), false));

            match result {
                Ok(result) => {
                    let vec = result
                        .into_iter()
                        .map(Dynamic::cast::<Id>)
                        .collect::<Vec<_>>();

                    if !vec.is_empty() {
                        state.ui_state.research_puzzle_selections = Some((selected, vec));
                    }

                    state.ui_state.selected_research_puzzle_tile = None;
                }
                Err(err) => {
                    rhai_log_err("selection_at_coord", &metadata.str_id, &err, None);
                    state.ui_state.research_puzzle_selections = None;
                }
            }
        }
    }

    if let Some((data, ..)) = &mut state.puzzle_state {
        if let Some(Data::TileMap(tiles)) = data
            .get_mut(state.resource_man.registry.data_ids.tiles)
            .cloned()
        {
            const BOARD_SIZE: Vec2 = Vec2::new(200.0, 200.0);

            let mut clicked = false;

            Pad::vertical(PADDING_MEDIUM).show(|| {
                group(|| {
                    scroll_horizontal_bar_alignment(Vec2::ZERO, BOARD_SIZE, None, || {
                        scroll_vertical_bar_alignment(Vec2::ZERO, BOARD_SIZE, None, || {
                            board_pos = PositionRecord::new()
                                .show(|| {
                                    constrained(
                                        Constraints {
                                            min: BOARD_SIZE,
                                            max: Vec2::INFINITY,
                                        },
                                        || {
                                            clicked = research_board_tiles(state, tiles);
                                        },
                                    );
                                })
                                .into_inner();
                        });
                    });
                });
            });

            if !completed && clicked {
                if let Some(min) = board_pos {
                    let p = state.input_handler.main_pos - min - PUZZLE_HEX_GRID_LAYOUT.hex_size;

                    let p = TileCoord::from(PUZZLE_HEX_GRID_LAYOUT.world_pos_to_hex(p));

                    state.ui_state.selected_research_puzzle_tile = Some(p);
                }
            }
        }
    }

    board_pos
}

pub fn player(state: &mut GameState, game_data: &mut DataMap) {
    if let Some(research) = state
        .ui_state
        .selected_research
        .and_then(|id| state.resource_man.get_research(id))
    {
        let mut a = false;
        let mut b = false;
        let mut ab = false;

        {
            game_data
                .entry(state.resource_man.registry.data_ids.research_items_filled)
                .or_insert_with(|| Data::SetId(Default::default()));

            game_data
                .entry(
                    state
                        .resource_man
                        .registry
                        .data_ids
                        .research_puzzle_completed,
                )
                .or_insert_with(|| Data::SetId(Default::default()));

            if let Some((Data::SetId(filled_items), Data::SetId(completed_puzzles))) = game_data
                .get(state.resource_man.registry.data_ids.research_items_filled)
                .zip(
                    game_data.get(
                        state
                            .resource_man
                            .registry
                            .data_ids
                            .research_puzzle_completed,
                    ),
                )
            {
                a = research.attached_puzzle.is_none() && filled_items.contains(&research.id);

                b = research.required_items.is_none() && completed_puzzles.contains(&research.id);

                ab =
                    filled_items.contains(&research.id) && completed_puzzles.contains(&research.id);
            }
        }

        if a || b || ab {
            if let Some(Data::SetId(set)) =
                game_data.get_mut(state.resource_man.registry.data_ids.research_items_filled)
            {
                set.remove(&research.id);
            }

            if let Some(Data::SetId(set)) = game_data.get_mut(
                state
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

            state.ui_state.selected_research_puzzle_tile = None;
            state.ui_state.research_puzzle_selections = None;
            state.ui_state.force_show_puzzle = true;
        }
    }

    let mut board_pos = None;

    Layer::new().show(|| {
        if !state.input_handler.key_active(ActionType::Player) {
            return;
        }

        let mut pos = state.ui_state.player_ui_position;
        movable(&mut pos, || {
            window_box(
                state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.player_menu)
                    .to_string(),
                || {
                    col(|| {
                        {
                            let mut row = list_row();
                            row.item_spacing = PADDING_MEDIUM;
                            row
                        }
                        .show(|| {
                            col(|| {
                                player_inventory(state, game_data);
                            });

                            col(|| {
                                research_selection(state, game_data);
                            });
                        });

                        col(|| {
                            current_research(state, game_data);
                        });

                        row(|| {
                            col(|| {
                                board_pos = research_puzzle(state, game_data);
                            });

                            Pad::horizontal(PADDING_MEDIUM).show(|| {
                                col(|| {
                                    if let Some(id) = state.ui_state.selected_research {
                                        if game_data.contains_id(
                                            state
                                                .resource_man
                                                .registry
                                                .data_ids
                                                .unlocked_researches,
                                            id,
                                        ) {
                                            if let Some(research) =
                                                state.resource_man.get_research(id)
                                            {
                                                divider(
                                                    BACKGROUND_3,
                                                    DIVIER_HEIGHT,
                                                    DIVIER_THICKNESS,
                                                );

                                                scroll_vertical(
                                                    Vec2::ZERO,
                                                    Vec2::new(460.0, 130.0),
                                                    || {
                                                        group(|| {
                                                            label(
                                                                &state.resource_man.research_str(
                                                                    research.completed_description,
                                                                ),
                                                            );
                                                        });
                                                    },
                                                );
                                            }
                                        }
                                    }
                                });
                            });
                        });
                    });
                },
            );
        });
        state.ui_state.player_ui_position = pos;
    });

    if let Some((data, dirty)) = &mut state.puzzle_state {
        if let Some(Data::TileMap(tiles)) = data.get_mut(state.resource_man.registry.data_ids.tiles)
        {
            let mut select_result = None;

            if let Some((coord, ids)) = &state.ui_state.research_puzzle_selections {
                if let Some(min) = board_pos {
                    let p = (PUZZLE_HEX_GRID_LAYOUT.hex_to_world_pos(**coord) + min).round();

                    Layer::new().show(|| {
                        reflow(
                            Alignment::TOP_LEFT,
                            Pivot::BOTTOM_LEFT,
                            Dim2::pixels(p.x + 20.0, p.y + 20.0),
                            || {
                                RoundRect::new(8.0, colors::WHITE).show_children(|| {
                                    Pad::all(PADDING_MEDIUM).show(|| {
                                        scroll_horizontal(
                                            Vec2::ZERO,
                                            Vec2::new(200.0, f32::INFINITY),
                                            || {
                                                row(|| {
                                                    let reset = interactive(|| {
                                                        ui_game_object(
                                                            InstanceData::default(),
                                                            UiGameObjectType::Model(ModelId(
                                                                state
                                                                    .resource_man
                                                                    .registry
                                                                    .model_ids
                                                                    .puzzle_space,
                                                            )),
                                                            PUZZLE_HEX_GRID_LAYOUT.hex_size * 2.0,
                                                            Some(IconMode::Item.model_matrix()),
                                                            Some(IconMode::Item.world_matrix()),
                                                        );
                                                    });

                                                    if reset.clicked {
                                                        select_result = Some((
                                                            *coord,
                                                            state
                                                                .resource_man
                                                                .registry
                                                                .model_ids
                                                                .puzzle_space,
                                                        ));
                                                        *dirty = true;
                                                    }

                                                    for id in ids {
                                                        let select = interactive(|| {
                                                            ui_game_object(
                                                                InstanceData::default(),
                                                                UiGameObjectType::Model(
                                                                    state
                                                                        .resource_man
                                                                        .model_or_missing_item(
                                                                            &ModelId(*id),
                                                                        ),
                                                                ),
                                                                PUZZLE_HEX_GRID_LAYOUT.hex_size
                                                                    * 2.0,
                                                                Some(IconMode::Item.model_matrix()),
                                                                Some(IconMode::Item.world_matrix()),
                                                            );
                                                        });

                                                        if select.clicked {
                                                            select_result = Some((*coord, *id));
                                                            *dirty = true;
                                                        }
                                                    }
                                                });
                                            },
                                        );
                                    });
                                });
                            },
                        );
                    });
                }
            }

            if let Some((selected, id)) = select_result {
                tiles.insert(selected, id);

                state.ui_state.selected_research_puzzle_tile = None;
                state.ui_state.research_puzzle_selections = None;
            }
        }
    }
}
