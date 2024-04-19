use std::mem;

use rhai::Dynamic;

use automancy_defs::glam::{dvec3, vec3, Vec2};
use automancy_defs::graph::visit::Topo;
use automancy_defs::hexx::{HexLayout, HexOrientation};
use automancy_defs::id::Id;
use automancy_defs::math;
use automancy_defs::rendering::InstanceData;
use automancy_defs::{coord::TileCoord, glam::vec2};
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::types::function::RhaiDataMap;
use automancy_resources::types::IconMode;
use automancy_resources::{rhai_call_options, rhai_log_err};
use yakui::{column, row, use_state, widgets::Absolute, Alignment, Dim2, Pivot, Rect};

use crate::gui::item::draw_item;
use crate::gui::{take_item_animation, MEDIUM_ICON_SIZE, SMALLISH_ICON_SIZE, SMALL_ICON_SIZE};
use crate::util::is_research_unlocked;
use crate::GameState;

use super::{
    components::{
        button::button,
        container::{group, window},
        interactive::interactive,
        position::PositionRecord,
        relative::Relative,
        scrollable::scroll_vertical,
        text::{heading, label},
    },
    ui_game_object,
};

const PUZZLE_HEX_GRID_LAYOUT: HexLayout = HexLayout {
    orientation: HexOrientation::Pointy,
    origin: Vec2::ZERO,
    hex_size: automancy_defs::glam::vec2(SMALLISH_ICON_SIZE, SMALLISH_ICON_SIZE),
    invert_x: false,
    invert_y: true,
};

pub fn player(state: &mut GameState, game_data: &mut DataMap) {
    window(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.player_menu]
            .to_string(),
        || {
            row(|| {
                column(|| {
                    heading(
                        state.resource_man.translates.gui
                            [&state.resource_man.registry.gui_ids.player_inventory_title]
                            .as_str(),
                    );

                    if let Some(Data::Inventory(inventory)) =
                        game_data.get(&state.resource_man.registry.data_ids.player_inventory)
                    {
                        scroll_vertical(200.0, || {
                            for (id, amount) in inventory.iter() {
                                let amount = *amount;

                                if amount != 0 {
                                    if let Some(item) =
                                        state.resource_man.registry.items.get(id).cloned()
                                    {
                                        Absolute::new(
                                            Alignment::TOP_LEFT,
                                            Pivot::TOP_LEFT,
                                            Vec2::ZERO,
                                        )
                                        .show(|| {
                                            let mut pos = PositionRecord::new().show(|| {
                                                draw_item(
                                                    &state.resource_man,
                                                    None,
                                                    ItemStack { item, amount },
                                                    MEDIUM_ICON_SIZE,
                                                    true,
                                                );
                                            });

                                            take_item_animation(
                                                state,
                                                item,
                                                Rect::from_pos_size(
                                                    pos.into_inner(),
                                                    vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                                                ),
                                            );
                                        });
                                    }
                                }
                            }
                        });
                    }
                });

                column(|| {
                    heading(
                        state.resource_man.translates.gui
                            [&state.resource_man.registry.gui_ids.research_menu_title]
                            .as_str(),
                    );

                    row(|| {
                        group(|| {
                            scroll_vertical(200.0, || {
                                let mut visitor =
                                    Topo::new(&state.resource_man.registry.researches);

                                column(|| {
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

                                        if interactive(|| {
                                            row(|| {
                                                ui_game_object(
                                                    InstanceData::default()
                                                        .with_model_matrix(
                                                            research.icon_mode.model_matrix(),
                                                        )
                                                        .with_world_matrix(
                                                            research.icon_mode.world_matrix(),
                                                        )
                                                        .with_light_pos(vec3(0.0, 4.0, 14.0), None),
                                                    icon,
                                                    vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                                                );

                                                label(
                                                    &state
                                                        .resource_man
                                                        .research_str(&research.name),
                                                );
                                            });
                                        })
                                        .clicked
                                        {
                                            state.gui_state.selected_research = Some(research.id);
                                            state.gui_state.selected_research_puzzle_tile = None;
                                            state.gui_state.research_puzzle_selections = None;
                                            state.puzzle_state = None; // TODO have a better save system for this
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
                            column(|| {
                                column(|| {
                                    heading(&state.resource_man.research_str(&research.name));
                                    label(&state.resource_man.research_str(&research.description));

                                    let mut already_unlocked = false;
                                    if let Some(Data::SetId(unlocked)) = game_data.get(
                                        &state.resource_man.registry.data_ids.unlocked_researches,
                                    ) {
                                        already_unlocked = unlocked.contains(&research.id)
                                    }

                                    if !already_unlocked {
                                        scroll_vertical(200.0, || {
                                            if let Some(stacks) = &research.required_items {
                                                for stack in stacks {
                                                    draw_item(
                                                        &state.resource_man,
                                                        None,
                                                        *stack,
                                                        SMALL_ICON_SIZE,
                                                        true,
                                                    );
                                                }
                                            }
                                        });

                                        let mut already_filled = false;
                                        if let Some(Data::SetId(items_filled)) = game_data.get(
                                            &state
                                                .resource_man
                                                .registry
                                                .data_ids
                                                .research_items_filled,
                                        ) {
                                            already_filled = items_filled.contains(&research.id)
                                        }

                                        if let Some(stacks) = &research.required_items {
                                            if button(
                                                &state.resource_man.translates.gui[&state
                                                    .resource_man
                                                    .registry
                                                    .gui_ids
                                                    .research_submit_items],
                                            )
                                            .clicked
                                                && !already_filled
                                            {
                                                let mut can_take = false;
                                                if let Some(Data::Inventory(inventory)) = game_data
                                                    .get_mut(
                                                        &state
                                                            .resource_man
                                                            .registry
                                                            .data_ids
                                                            .player_inventory,
                                                    )
                                                {
                                                    can_take = stacks
                                                        .iter()
                                                        .all(|v| inventory.contains(*v))
                                                }

                                                if can_take {
                                                    if let Some(Data::Inventory(inventory)) =
                                                        game_data.get_mut(
                                                            &state
                                                                .resource_man
                                                                .registry
                                                                .data_ids
                                                                .player_inventory,
                                                        )
                                                    {
                                                        for stack in stacks {
                                                            inventory
                                                                .take(stack.item.id, stack.amount);
                                                        }
                                                    }

                                                    if let Some(Data::SetId(items_filled)) =
                                                        game_data.get_mut(
                                                            &state
                                                                .resource_man
                                                                .registry
                                                                .data_ids
                                                                .research_items_filled,
                                                        )
                                                    {
                                                        items_filled.insert(research.id);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });

                                let show_research_puzzle = {
                                    let research = state
                                        .gui_state
                                        .selected_research
                                        .and_then(|id| state.resource_man.get_research(id))
                                        .unwrap();

                                    if game_data
                                        .get(
                                            &state
                                                .resource_man
                                                .registry
                                                .data_ids
                                                .research_puzzle_completed,
                                        )
                                        .and_then(|v| match v {
                                            Data::SetId(set) => Some(set.contains(&research.id)),
                                            _ => None,
                                        })
                                        .unwrap_or(false)
                                    {
                                        false
                                    } else if research.required_items.is_some() {
                                        game_data
                                            .get(
                                                &state
                                                    .resource_man
                                                    .registry
                                                    .data_ids
                                                    .research_items_filled,
                                            )
                                            .and_then(|filled_items| match filled_items {
                                                Data::SetId(set) => {
                                                    Some(set.contains(&research.id))
                                                }
                                                _ => None,
                                            })
                                            .unwrap_or(false)
                                    } else {
                                        true
                                    }
                                };

                                if show_research_puzzle {
                                    if let Some(((ast, default_scope, function_id), setup)) = state
                                        .gui_state
                                        .selected_research
                                        .and_then(|id| state.resource_man.get_research(id))
                                        .and_then(|research| research.attached_puzzle.as_ref())
                                        .and_then(|(id, setup)| {
                                            state.resource_man.functions.get(id).zip(Some(setup))
                                        })
                                    {
                                        let mut scope = default_scope.clone_visible();

                                        let puzzle_state =
                                            state.puzzle_state.get_or_insert_with(|| {
                                                let data = RhaiDataMap::default();
                                                let mut rhai_state = Dynamic::from(data);

                                                let result = state
                                                    .resource_man
                                                    .engine
                                                    .call_fn_with_options::<()>(
                                                        rhai_call_options(&mut rhai_state),
                                                        &mut scope,
                                                        ast,
                                                        "pre_setup",
                                                        (Dynamic::from(setup.clone()),),
                                                    );

                                                match result {
                                                    Err(err) => rhai_log_err(function_id, &err),
                                                    _ => {}
                                                }

                                                (rhai_state.take().cast::<RhaiDataMap>(), true)
                                            });

                                        if puzzle_state.1 {
                                            let mut rhai_state =
                                                Dynamic::from(mem::take(&mut puzzle_state.0));

                                            let result = state
                                                .resource_man
                                                .engine
                                                .call_fn_with_options::<bool>(
                                                    rhai_call_options(&mut rhai_state),
                                                    &mut scope,
                                                    ast,
                                                    "evaluate",
                                                    (Dynamic::from(setup.clone()),),
                                                );

                                            *puzzle_state =
                                                (rhai_state.take().cast::<RhaiDataMap>(), false);

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
                                                            .or_insert_with(|| {
                                                                Data::SetId(Default::default())
                                                            })
                                                        {
                                                            set.insert(research.id);
                                                        }
                                                    }
                                                }
                                                Err(err) => rhai_log_err(function_id, &err),
                                            }
                                        }

                                        if let Some(selected) =
                                            state.gui_state.selected_research_puzzle_tile
                                        {
                                            let mut rhai_state =
                                                Dynamic::from(mem::take(&mut puzzle_state.0));

                                            let result = state
                                                .resource_man
                                                .engine
                                                .call_fn_with_options::<Dynamic>(
                                                    rhai_call_options(&mut rhai_state),
                                                    &mut scope,
                                                    ast,
                                                    "selection_at_coord",
                                                    (Dynamic::from(setup.clone()), selected),
                                                );

                                            state.puzzle_state = Some((
                                                rhai_state.take().cast::<RhaiDataMap>(),
                                                false,
                                            ));

                                            match result {
                                                Ok(result) => {
                                                    if let Some(vec) = result.try_cast::<Vec<Id>>()
                                                    {
                                                        if !vec.is_empty() {
                                                            state
                                                                .gui_state
                                                                .research_puzzle_selections =
                                                                Some((selected, vec));
                                                        }
                                                    }

                                                    state.gui_state.selected_research_puzzle_tile =
                                                        None;
                                                }
                                                Err(err) => rhai_log_err(function_id, &err),
                                            }
                                        }

                                        if let Some((data, dirty)) = &mut state.puzzle_state {
                                            if let Some(Data::TileMap(tiles)) = data
                                                .get_mut(state.resource_man.registry.data_ids.tiles)
                                            {
                                                let mut min = Vec2::MIN;

                                                let pos = use_state(|| Vec2::ZERO);
                                                let response = interactive(|| {
                                                    let new_pos =
                                                        PositionRecord::new().show(|| {
                                                            group(|| {
                                                                // let offset = vec2(PUZZLE_HEX_GRID_LAYOUT.hex_size.x / 2.0, 0.0);
                                                                let offset = Vec2::ZERO;

                                                                for (coord, id) in tiles.iter() {
                                                                    let [x, y] =
                                                                        PUZZLE_HEX_GRID_LAYOUT
                                                                            .hex_to_world_pos(
                                                                                **coord,
                                                                            )
                                                                            .to_array();

                                                                    let pos = vec2(
                                                                        (offset.x + x / 2.0)
                                                                            .round(),
                                                                        (offset.y + y / 2.0)
                                                                            .round(),
                                                                    );
                                                                    min = min.min(pos);

                                                                    Relative::new(
                                                                        Alignment::TOP_LEFT,
                                                                        Pivot::TOP_LEFT,
                                                                        Dim2::pixels(min.x, min.y),
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
                                                                    state
                                                                        .resource_man
                                                                        .get_item_model(
                                                                            state
                                                                                .resource_man
                                                                                .get_puzzle_model(
                                                                                    *id,
                                                                                ),
                                                                        ),
                                                                    vec2(
                                                                        PUZZLE_HEX_GRID_LAYOUT
                                                                            .hex_size
                                                                            .x,
                                                                        PUZZLE_HEX_GRID_LAYOUT
                                                                            .hex_size
                                                                            .y,
                                                                    ),
                                                                );
                                                                    });
                                                                }
                                                            });
                                                        });

                                                    pos.set(new_pos.into_inner());
                                                });

                                                let mut select_result = None;

                                                if let Some((selected, ids)) =
                                                    &state.gui_state.research_puzzle_selections
                                                {
                                                    Absolute::new(
                                                        Alignment::TOP_LEFT,
                                                        Pivot::TOP_LEFT,
                                                        pos.get()
                                                            + PUZZLE_HEX_GRID_LAYOUT
                                                                .hex_to_world_pos(**selected),
                                                    )
                                                    .show(|| {
                                                        scroll_vertical(200.0, || {
                                                            row(|| {
                                                                if interactive(|| {
                                                                    ui_game_object(
                                                                        InstanceData::default()
                                                                            .with_world_matrix(
                                                                                math::view(dvec3(
                                                                                    0.0, 0.0, 1.0,
                                                                                ))
                                                                                .as_mat4(),
                                                                            ),
                                                                        state
                                                                            .resource_man
                                                                            .registry
                                                                            .model_ids
                                                                            .puzzle_space,
                                                                        vec2(
                                                                            SMALLISH_ICON_SIZE,
                                                                            SMALLISH_ICON_SIZE,
                                                                        ),
                                                                    );
                                                                })
                                                                .clicked
                                                                {
                                                                    select_result = Some((
                                                                        *selected,
                                                                        state
                                                                            .resource_man
                                                                            .registry
                                                                            .model_ids
                                                                            .puzzle_space,
                                                                    ));
                                                                    *dirty = true;
                                                                }

                                                                for id in ids {
                                                                    if interactive(|| {
                                                                        ui_game_object(
                                                                            InstanceData::default()
                                                                                .with_world_matrix(
                                                                                    math::view(
                                                                                        dvec3(
                                                                                            0.0,
                                                                                            0.0,
                                                                                            1.0,
                                                                                        ),
                                                                                    )
                                                                                    .as_mat4(),
                                                                                ),
                                                                            state
                                                                                .resource_man
                                                                                .get_item_model(
                                                                                state
                                                                                    .resource_man
                                                                                    .registry
                                                                                    .items[id]
                                                                                    .model,
                                                                            ),
                                                                            vec2(
                                                                                SMALLISH_ICON_SIZE,
                                                                                SMALLISH_ICON_SIZE,
                                                                            ),
                                                                        );
                                                                    })
                                                                    .clicked
                                                                    {
                                                                        select_result =
                                                                            Some((*selected, *id));
                                                                        *dirty = true;
                                                                    }
                                                                }
                                                            });
                                                        });
                                                    });
                                                }

                                                if let Some((selected, id)) = select_result {
                                                    tiles.insert(selected, id);

                                                    state.gui_state.selected_research_puzzle_tile =
                                                        None;
                                                    state.gui_state.research_puzzle_selections =
                                                        None;
                                                }

                                                if response.clicked {
                                                    let p = state.input_handler.main_pos.as_vec2()
                                                        - min;

                                                    let p = p * 2.0
                                                        - vec2(
                                                            PUZZLE_HEX_GRID_LAYOUT.hex_size.x * 2.0,
                                                            PUZZLE_HEX_GRID_LAYOUT.hex_size.y,
                                                        );

                                                    let p = TileCoord::from(
                                                        PUZZLE_HEX_GRID_LAYOUT
                                                            .world_pos_to_hex(vec2(p.x, p.y)),
                                                    );

                                                    state.gui_state.selected_research_puzzle_tile =
                                                        Some(p);
                                                }
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
                                    .get(
                                        &state.resource_man.registry.data_ids.research_items_filled,
                                    )
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

                                state.gui_state.selected_research_puzzle_tile = None;
                                state.gui_state.research_puzzle_selections = None;
                            }

                            if let Some(Data::SetId(set)) = game_data
                                .get(&state.resource_man.registry.data_ids.unlocked_researches)
                            {
                                let research = &state
                                    .gui_state
                                    .selected_research
                                    .and_then(|id| state.resource_man.get_research(id))
                                    .unwrap();

                                if set.contains(&research.id) {
                                    label(
                                        &state
                                            .resource_man
                                            .research_str(&research.completed_description),
                                    );
                                }
                            }
                        }
                    });
                });
            });
        },
    );
}
