use std::mem;

use rhai::Dynamic;

use automancy_defs::graph::visit::Topo;
use automancy_defs::hexx::{HexLayout, HexOrientation};
use automancy_defs::id::Id;
use automancy_defs::math;
use automancy_defs::rendering::InstanceData;
use automancy_defs::{
    colors,
    glam::{dvec3, vec3, Vec2},
};
use automancy_defs::{coord::TileCoord, glam::vec2};
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::types::function::RhaiDataMap;
use automancy_resources::types::IconMode;
use automancy_resources::{rhai_call_options, rhai_log_err};
use yakui::{
    column, constrained, divider, row,
    widgets::{Layer, Pad},
    Alignment, Constraints, Dim2, Pivot, Rect,
};

use crate::GameState;
use crate::{input::ActionType, util::is_research_unlocked};

use super::{
    button, centered_row, group, heading, inactive_button, interactive, item::draw_item, label,
    movable, scroll_horizontal, scroll_vertical, ui_game_object, util::take_item_animation,
    window_box, AbsoluteRect, PositionRecord, Relative, RoundRect, DIVIER_SIZE, MEDIUM_ICON_SIZE,
    PADDING_MEDIUM, SMALLISH_ICON_SIZE, SMALL_ICON_SIZE,
};

const PUZZLE_HEX_GRID_LAYOUT: HexLayout = HexLayout {
    orientation: HexOrientation::Pointy,
    origin: Vec2::ZERO,
    hex_size: automancy_defs::glam::vec2(SMALLISH_ICON_SIZE, SMALLISH_ICON_SIZE),
    invert_x: false,
    invert_y: true,
};

fn hex_to_board_pixels(coord: TileCoord) -> Vec2 {
    let offset = vec2(PUZZLE_HEX_GRID_LAYOUT.hex_size.x / 2.0, 0.0);
    let [x, y] = PUZZLE_HEX_GRID_LAYOUT.hex_to_world_pos(*coord).to_array();

    vec2(offset.x + x / 2.0, offset.y + y / 2.0)
}

fn player_inventory(state: &mut GameState, game_data: &mut DataMap) {
    heading(
        state
            .resource_man
            .gui_str(&state.resource_man.registry.gui_ids.player_inventory_title)
            .as_str(),
    );

    let Some(Data::Inventory(inventory)) =
        game_data.get(&state.resource_man.registry.data_ids.player_inventory)
    else {
        return;
    };

    scroll_vertical(200.0, || {
        column(|| {
            for (id, amount) in inventory.iter() {
                let amount = *amount;

                if amount != 0 {
                    if let Some(item) = state.resource_man.registry.items.get(id).cloned() {
                        let rect = draw_item(
                            &state.resource_man,
                            || {},
                            ItemStack { item, amount },
                            MEDIUM_ICON_SIZE,
                            true,
                        );

                        if let Some(rect) = rect {
                            take_item_animation(
                                state,
                                item,
                                Rect::from_pos_size(
                                    rect.pos(),
                                    vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                                ),
                            );
                        }
                    }
                }
            }
        });
    });
}

fn research_selection(state: &mut GameState, game_data: &mut DataMap) {
    heading(
        state
            .resource_man
            .gui_str(&state.resource_man.registry.gui_ids.research_menu_title)
            .as_str(),
    );

    let mut visitor = Topo::new(&state.resource_man.registry.researches);

    scroll_vertical(200.0, || {
        column(|| {
            while let Some(idx) = visitor.next(&state.resource_man.registry.researches) {
                let research = &state.resource_man.registry.researches[idx];
                let icon = match research.icon_mode {
                    IconMode::Item => state.resource_man.get_item_model(research.icon),
                    IconMode::Tile => state.resource_man.get_model(research.icon),
                };

                if let Some(prev) = research.depends_on {
                    if !is_research_unlocked(prev, &state.resource_man, game_data) {
                        continue;
                    }
                }

                let interact = interactive(|| {
                    centered_row(|| {
                        ui_game_object(
                            InstanceData::default()
                                .with_model_matrix(research.icon_mode.model_matrix())
                                .with_world_matrix(research.icon_mode.world_matrix())
                                .with_light_pos(vec3(0.0, 1.0, 8.0), None),
                            icon,
                            vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                        );

                        label(&state.resource_man.research_str(&research.name));
                    });
                });

                if interact.clicked {
                    state.gui_state.selected_research = Some(research.id);
                    state.gui_state.selected_research_puzzle_tile = None;
                    state.gui_state.research_puzzle_selections = None;
                    state.puzzle_state = None; // TODO have a better save system for this
                    state.gui_state.force_show_puzzle = false;
                };
            }
        });
    });
}

fn current_research(state: &mut GameState, game_data: &mut DataMap) {
    let Some(research) = state
        .gui_state
        .selected_research
        .and_then(|id| state.resource_man.get_research(id))
    else {
        return;
    };

    heading(&state.resource_man.research_str(&research.name));
    label(&state.resource_man.research_str(&research.description));

    let mut already_unlocked = false;
    if let Some(Data::SetId(unlocked)) =
        game_data.get(&state.resource_man.registry.data_ids.unlocked_researches)
    {
        already_unlocked = unlocked.contains(&research.id)
    }

    if !already_unlocked {
        let mut already_filled = false;
        if let Some(Data::SetId(items_filled)) =
            game_data.get(&state.resource_man.registry.data_ids.research_items_filled)
        {
            already_filled = items_filled.contains(&research.id)
        }

        scroll_vertical(200.0, || {
            column(|| {
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
                .gui_str(&state.resource_man.registry.gui_ids.research_submit_items);

            let submit_button = if !already_filled {
                button(submit_text)
            } else {
                inactive_button(submit_text)
            };

            if !already_filled && submit_button.clicked {
                let mut can_take = false;

                if let Some(Data::Inventory(inventory)) =
                    game_data.get_mut(&state.resource_man.registry.data_ids.player_inventory)
                {
                    can_take = stacks.iter().all(|v| inventory.contains(*v))
                }

                if can_take {
                    if let Some(Data::Inventory(inventory)) =
                        game_data.get_mut(&state.resource_man.registry.data_ids.player_inventory)
                    {
                        for stack in stacks {
                            inventory.take(stack.item.id, stack.amount);
                        }
                    }

                    if let Some(Data::SetId(items_filled)) = game_data
                        .get_mut(&state.resource_man.registry.data_ids.research_items_filled)
                    {
                        items_filled.insert(research.id);
                    }
                }
            }
        }
    }
}

fn research_puzzle(state: &mut GameState, game_data: &mut DataMap) -> Option<Vec2> {
    let research = state
        .gui_state
        .selected_research
        .and_then(|id| state.resource_man.get_research(id))?;

    let completed = game_data
        .get(
            &state
                .resource_man
                .registry
                .data_ids
                .research_puzzle_completed,
        )
        .and_then(|completed| match completed {
            Data::SetId(set) => Some(set.contains(&research.id)),
            _ => None,
        })
        .unwrap_or(false);

    if !state.gui_state.force_show_puzzle {
        if research.required_items.is_some()
            && game_data
                .get(&state.resource_man.registry.data_ids.research_items_filled)
                .and_then(|filled_items| match filled_items {
                    Data::SetId(set) => Some(!set.contains(&research.id)),
                    _ => None,
                })
                .unwrap_or(true)
        {
            return None;
        }

        if completed {
            return None;
        }
    }

    let mut board_pos = None;
    if let Some(((ast, default_scope, function_id), setup)) = state
        .gui_state
        .selected_research
        .and_then(|id| state.resource_man.get_research(id))
        .and_then(|research| research.attached_puzzle.as_ref())
        .and_then(|(id, setup)| state.resource_man.functions.get(id).zip(Some(setup)))
    {
        let mut scope = default_scope.clone_visible();

        let puzzle_state = state.puzzle_state.get_or_insert_with(|| {
            let data = RhaiDataMap::default();
            let mut rhai_state = Dynamic::from(data);

            let result = state.resource_man.engine.call_fn_with_options::<()>(
                rhai_call_options(&mut rhai_state),
                &mut scope,
                ast,
                "pre_setup",
                (Dynamic::from(setup.clone()),),
            );

            if let Err(err) = result {
                rhai_log_err(function_id, &err)
            }

            (rhai_state.take().cast::<RhaiDataMap>(), true)
        });

        if puzzle_state.1 {
            let mut rhai_state = Dynamic::from(mem::take(&mut puzzle_state.0));

            let result = state.resource_man.engine.call_fn_with_options::<bool>(
                rhai_call_options(&mut rhai_state),
                &mut scope,
                ast,
                "evaluate",
                (Dynamic::from(setup.clone()),),
            );

            *puzzle_state = (rhai_state.take().cast::<RhaiDataMap>(), false);

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
                Err(err) => rhai_log_err(function_id, &err),
            }
        }

        if let Some(selected) = state.gui_state.selected_research_puzzle_tile {
            let mut rhai_state = Dynamic::from(mem::take(&mut puzzle_state.0));

            let result = state.resource_man.engine.call_fn_with_options::<Dynamic>(
                rhai_call_options(&mut rhai_state),
                &mut scope,
                ast,
                "selection_at_coord",
                (Dynamic::from(setup.clone()), selected),
            );

            state.puzzle_state = Some((rhai_state.take().cast::<RhaiDataMap>(), false));

            match result {
                Ok(result) => {
                    if let Some(vec) = result.try_cast::<Vec<Id>>() {
                        if !vec.is_empty() {
                            state.gui_state.research_puzzle_selections = Some((selected, vec));
                        }
                    } else {
                        state.gui_state.research_puzzle_selections = None;
                    }

                    state.gui_state.selected_research_puzzle_tile = None;
                }
                Err(err) => rhai_log_err(function_id, &err),
            }
        }

        if let Some((data, ..)) = &mut state.puzzle_state {
            if let Some(Data::TileMap(tiles)) =
                data.get_mut(state.resource_man.registry.data_ids.tiles)
            {
                const BOARD_SIZE: f32 = 200.0;

                let mut clicked = false;

                group(|| {
                    scroll_horizontal(BOARD_SIZE, || {
                        scroll_vertical(BOARD_SIZE, || {
                            let pos = PositionRecord::new().show(|| {
                                constrained(
                                    Constraints::tight(vec2(BOARD_SIZE, BOARD_SIZE)),
                                    || {
                                        let interact = interactive(|| {
                                            column(|| {
                                                for (coord, id) in tiles.iter() {
                                                    let pos =
                                                        hex_to_board_pixels(*coord) / BOARD_SIZE;

                                                    Relative::new(
                                                        Alignment::new(pos.x, pos.y),
                                                        Pivot::TOP_LEFT,
                                                        Dim2::ZERO,
                                                    )
                                                    .show(|| {
                                                        ui_game_object(
                                                            InstanceData::default()
                                                                .with_world_matrix(
                                                                    math::view(dvec3(
                                                                        0.0, 0.0, 1.0,
                                                                    ))
                                                                    .as_mat4(),
                                                                ),
                                                            state.resource_man.get_item_model(
                                                                state
                                                                    .resource_man
                                                                    .get_puzzle_model(*id),
                                                            ),
                                                            vec2(
                                                                PUZZLE_HEX_GRID_LAYOUT.hex_size.x,
                                                                PUZZLE_HEX_GRID_LAYOUT.hex_size.y,
                                                            ),
                                                        );
                                                    });
                                                }
                                            });
                                        });

                                        clicked = interact.clicked;
                                    },
                                );
                            });
                            board_pos = Some(pos.into_inner());
                        });
                    });
                });

                if !completed && clicked {
                    if let Some(min) = board_pos {
                        let p = state.input_handler.main_pos.as_vec2() - min;

                        let p = p * 2.0
                            - vec2(
                                PUZZLE_HEX_GRID_LAYOUT.hex_size.x * 2.0,
                                PUZZLE_HEX_GRID_LAYOUT.hex_size.y,
                            );

                        let p = TileCoord::from(
                            PUZZLE_HEX_GRID_LAYOUT.world_pos_to_hex(vec2(p.x, p.y)),
                        );

                        state.gui_state.selected_research_puzzle_tile = Some(p);
                    }
                }
            }
        }
    }

    board_pos
}

pub fn player(state: &mut GameState, game_data: &mut DataMap) {
    let mut board_pos = None;

    Layer::new().show(|| {
        if !state.input_handler.key_active(ActionType::Player) {
            return;
        }

        let mut pos = state.gui_state.player_ui_position;
        movable(&mut pos, || {
            window_box(
                state
                    .resource_man
                    .gui_str(&state.resource_man.registry.gui_ids.player_menu)
                    .to_string(),
                || {
                    column(|| {
                        row(|| {
                            group(|| {
                                player_inventory(state, game_data);
                            });

                            group(|| {
                                research_selection(state, game_data);
                            });
                        });

                        row(|| {
                            constrained(
                                Constraints::loose(Vec2::new(520.0, f32::INFINITY)),
                                || {
                                    column(|| {
                                        current_research(state, game_data);
                                    });
                                },
                            );
                        });

                        column(|| {
                            board_pos = research_puzzle(state, game_data);
                        });

                        row(|| {
                            if let Some(research) = &state
                                .gui_state
                                .selected_research
                                .and_then(|id| state.resource_man.get_research(id))
                            {
                                if let Some(Data::SetId(set)) = game_data
                                    .get(&state.resource_man.registry.data_ids.unlocked_researches)
                                {
                                    divider(colors::INACTIVE, DIVIER_SIZE, DIVIER_SIZE);
                                    scroll_vertical(200.0, || {
                                        constrained(
                                            Constraints::loose(Vec2::new(680.0, f32::INFINITY)),
                                            || {
                                                if set.contains(&research.id) {
                                                    label(&state.resource_man.research_str(
                                                        &research.completed_description,
                                                    ));
                                                }
                                            },
                                        );
                                    });
                                }
                            }
                        });
                    });
                },
            );
        });
        state.gui_state.player_ui_position = pos;
    });

    if let Some((data, dirty)) = &mut state.puzzle_state {
        if let Some(Data::TileMap(tiles)) = data.get_mut(state.resource_man.registry.data_ids.tiles)
        {
            let mut select_result = None;

            if let Some((coord, ids)) = &state.gui_state.research_puzzle_selections {
                if let Some(min) = board_pos {
                    let p = (hex_to_board_pixels(*coord) + min).round();

                    Layer::new().show(|| {
                        AbsoluteRect::new(p + vec2(20.0, 20.0), Pivot::BOTTOM_LEFT).show(|| {
                            RoundRect::new(8.0, colors::WHITE).show_children(|| {
                                Pad::all(PADDING_MEDIUM).show(|| {
                                    scroll_horizontal(200.0, || {
                                        centered_row(|| {
                                            let reset = interactive(|| {
                                                ui_game_object(
                                                    InstanceData::default().with_world_matrix(
                                                        math::view(dvec3(0.0, 0.0, 1.0)).as_mat4(),
                                                    ),
                                                    state
                                                        .resource_man
                                                        .registry
                                                        .model_ids
                                                        .puzzle_space,
                                                    vec2(SMALLISH_ICON_SIZE, SMALLISH_ICON_SIZE),
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
                                                        InstanceData::default().with_world_matrix(
                                                            math::view(dvec3(0.0, 0.0, 1.0))
                                                                .as_mat4(),
                                                        ),
                                                        state.resource_man.get_item_model(
                                                            state.resource_man.registry.items[id]
                                                                .model,
                                                        ),
                                                        vec2(
                                                            SMALLISH_ICON_SIZE,
                                                            SMALLISH_ICON_SIZE,
                                                        ),
                                                    );
                                                });

                                                if select.clicked {
                                                    select_result = Some((*coord, *id));
                                                    *dirty = true;
                                                }
                                            }
                                        });
                                    });
                                });
                            });
                        });
                    });
                }
            }

            if let Some((selected, id)) = select_result {
                tiles.insert(selected, id);

                state.gui_state.selected_research_puzzle_tile = None;
                state.gui_state.research_puzzle_selections = None;
            }
        }
    }

    if let Some(research) = state
        .gui_state
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
                a = research.attached_puzzle.is_none() && filled_items.contains(&research.id);

                b = research.required_items.is_none() && completed_puzzles.contains(&research.id);

                ab =
                    filled_items.contains(&research.id) && completed_puzzles.contains(&research.id);
            }
        }

        if a || b || ab {
            if let Some(Data::SetId(set)) =
                game_data.get_mut(&state.resource_man.registry.data_ids.research_items_filled)
            {
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

            state.gui_state.selected_research_puzzle_tile = None;
            state.gui_state.research_puzzle_selections = None;
            state.gui_state.force_show_puzzle = true;
        }
    }
}
