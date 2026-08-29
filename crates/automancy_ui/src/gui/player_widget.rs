use automancy_data::id_map::IdSet;
use automancy_game::scripting_rhai;
use petgraph::visit::Topo;
use rhai::{Dynamic, Scope};

use crate::*;

const PUZZLE_HEX_SIZE: Vec2 = Vec2::new(sizing::TINY_ICON_SIZE, sizing::TINY_ICON_SIZE);
const PUZZLE_HEX_GRID_LAYOUT: TileLayout = TileLayout {
    origin: vek::Vec2::new(PUZZLE_HEX_SIZE.x, 0.0),
    scale: vek::Vec2::new(PUZZLE_HEX_SIZE.x, PUZZLE_HEX_SIZE.y),
};

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn player_inventory(ctx: &mut UiContext, closed: bool) {
    Text::heading(gui_str!(ctx.game_state, player_inventory_title)).show();

    let inventory = ctx
        .game_data
        .map_data
        .stacks(ctx.game_state.resource_man.registry.data_ids.player_inventory);

    Scrollable::vertical()
        .child_size(Constraints::loose(Vec2::new(f32::INFINITY, 200.0)))
        .show(|| {
            section(|| {
                col(|| {
                    for (&id, &amount) in inventory.iter() {
                        if amount != 0 {
                            let rect = RectRecorder::new()
                                .show(|| {
                                    draw_item_stack(
                                        &ctx.game_state.resource_man,
                                        ItemStack {
                                            id,
                                            amount,
                                        },
                                        sizing::MEDIUM_ICON_SIZE,
                                    );
                                })
                                .into_inner();

                            if !closed {
                                item_animation_dst(
                                    ctx.game_state.resource_man.registry.render_ids.player_inventory_animation,
                                    id,
                                    rect.map(|rect| Rect::from_pos_size(rect.pos(), Vec2::splat(sizing::MEDIUM_ICON_SIZE))),
                                );
                            } else {
                                item_animation_dst(ctx.game_state.resource_man.registry.render_ids.player_inventory_animation, id, None);
                            }
                        }
                    }
                });
            });
        });
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn research_selection(ctx: &mut UiContext) {
    Text::heading(gui_str!(ctx.game_state, research_menu_title)).show();

    let mut visitor = Topo::new(&ctx.game_state.resource_man.registry.research_defs);

    Scrollable::vertical()
        .child_size(Constraints::loose(Vec2::new(f32::INFINITY, 200.0)))
        .show(|| {
            section(|| {
                col(|| {
                    while let Some(index) = visitor.next(&ctx.game_state.resource_man.registry.research_defs) {
                        let research = &ctx.game_state.resource_man.registry.research_defs[index];

                        if !research.depends_on.is_none()
                            && !ctx
                                .game_state
                                .resource_man
                                .is_research_unlocked(research.depends_on, &ctx.game_data.map_data)
                        {
                            continue;
                        }

                        let interact = interactive(|| {
                            row_center(|| {
                                GameModel::new(research.icon, Vec2::splat(sizing::MEDIUM_ICON_SIZE)).show();

                                Text::normal(ctx.game_state.resource_man.research_str(research.name).to_string()).show();
                            });
                        });

                        if interact.clicked {
                            ctx.gui.state.selected_research = Some(research.id);
                            ctx.gui.state.selected_research_puzzle_tile = None;
                            ctx.gui.state.research_puzzle_selections = None;
                            ctx.gui.state.force_show_puzzle = false;
                            ctx.gui.puzzle_state = None; // TODO have a better save system for this
                        };
                    }
                });
            });
        });
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn current_research(ctx: &mut UiContext) {
    let Some(research) = ctx
        .gui
        .state
        .selected_research
        .and_then(|id| ctx.game_state.resource_man.get_research(id))
    else {
        return;
    };

    Text::heading(ctx.game_state.resource_man.research_str(research.name).to_string()).show();

    constrained(Constraints::loose(Vec2::new(500.0, f32::INFINITY)), || {
        Text::normal(ctx.game_state.resource_man.research_str(research.description).to_string()).show();
    });

    if !ctx
        .game_data
        .map_data
        .contains_research_id(ctx.game_state.resource_man.registry.data_ids.unlocked_researches, research.id)
    {
        let already_filled = ctx
            .game_data
            .map_data
            .contains_research_id(ctx.game_state.resource_man.registry.data_ids.research_items_filled, research.id);

        Scrollable::vertical()
            .child_size(Constraints::loose(Vec2::new(240.0, 200.0)))
            .show(|| {
                col(|| {
                    if let Some(stacks) = &research.required_items {
                        for &stack in stacks {
                            draw_item_stack(&ctx.game_state.resource_man, stack, sizing::SMALL_ICON_SIZE);
                        }
                    }
                });
            });

        if let Some(stacks) = &research.required_items {
            let submit_text = gui_str!(ctx.game_state, btn_research_submit_items);

            let submit_button = if !already_filled {
                button(submit_text)
            } else {
                inactive_button(submit_text)
            };

            if !already_filled
                && submit_button.clicked
                && stacks.iter().all(|v| {
                    ctx.game_data
                        .map_data
                        .contains_stack(ctx.game_state.resource_man.registry.data_ids.player_inventory, *v)
                })
            {
                ctx.game_data.sub_map_datum(
                    ctx.game_state.resource_man.registry.data_ids.player_inventory,
                    Datum::Inventory(Inventory::from_iter(stacks.iter().copied())),
                );

                ctx.game_data.add_map_datum(
                    ctx.game_state.resource_man.registry.data_ids.research_items_filled,
                    Datum::ResearchId(research.id),
                );
            }
        }
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn research_board_tiles<'a>(game_state: &mut AutomancyGameState, tiles: impl IntoIterator<Item = (&'a TileCoord, &'a ModelId)>) -> bool {
    let interact = interactive(|| {
        col(|| {
            for (&coord, &id) in tiles.into_iter() {
                let pos = coord.to_world_pos_with(PUZZLE_HEX_GRID_LAYOUT);
                let model_id = game_state.resource_man.model_or_puzzle_space(id);

                reflow(Alignment::TOP_LEFT, Pivot::TOP_LEFT, Dim2::pixels(pos.x, pos.y), || {
                    GameModel::new(GenericModel::Plain(model_id), PUZZLE_HEX_SIZE * 2.0).show();
                });
            }
        });
    });
    interact.clicked
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn research_puzzle(ctx: &mut UiContext) -> Option<Rect> {
    let research = ctx
        .gui
        .state
        .selected_research
        .and_then(|id| ctx.game_state.resource_man.get_research(id))?;

    let completed = ctx
        .game_data
        .map_data
        .contains_research_id(ctx.game_state.resource_man.registry.data_ids.research_puzzle_completed, research.id);

    if !ctx.gui.state.force_show_puzzle {
        if research.required_items.is_some()
            && !ctx
                .game_data
                .map_data
                .contains_research_id(ctx.game_state.resource_man.registry.data_ids.research_items_filled, research.id)
        {
            return None;
        }

        if completed {
            return None;
        }
    }

    let mut board_rect = None;
    if let Some((script, setup)) = ctx
        .gui
        .state
        .selected_research
        .and_then(|id| ctx.game_state.resource_man.get_research(id))
        .and_then(|research| research.attached_puzzle.as_ref())
        .and_then(|(id, setup)| ctx.game_state.resource_man.rhai_scripts.get(id).zip(Some(setup)))
    {
        let puzzle_state = ctx.gui.puzzle_state.get_or_insert_with(|| {
            let mut rhai_state = Dynamic::from(DataMap::default());

            let result = ctx.game_state.resource_man.rhai.call_fn_with_options::<()>(
                scripting_rhai::rhai_call_options(&mut rhai_state),
                &mut Scope::default(),
                &script.ast,
                "pre_setup",
                (Dynamic::from(setup.clone()),),
            );

            if let Err(err) = result {
                scripting_rhai::rhai_log_err("pre_setup", &script.metadata.str_id, &err, None)
            }

            (rhai_state.take().cast::<DataMap>(), true)
        });

        if puzzle_state.1 {
            let mut rhai_state = Dynamic::from(std::mem::take(&mut puzzle_state.0));

            let result = ctx.game_state.resource_man.rhai.call_fn_with_options::<bool>(
                scripting_rhai::rhai_call_options(&mut rhai_state),
                &mut Scope::new(),
                &script.ast,
                "evaluate",
                (Dynamic::from(setup.clone()),),
            );

            *puzzle_state = (rhai_state.take().cast::<DataMap>(), false);

            match result {
                Ok(result) => {
                    if result {
                        ctx.game_data.add_map_datum(
                            ctx.game_state.resource_man.registry.data_ids.research_puzzle_completed,
                            Datum::ResearchId(research.id),
                        );
                    }
                },
                Err(err) => scripting_rhai::rhai_log_err("evaluate", &script.metadata.str_id, &err, None),
            }
        }

        if let Some(selected) = ctx.gui.state.selected_research_puzzle_tile {
            let mut rhai_state = Dynamic::from(std::mem::take(&mut puzzle_state.0));

            let result = ctx.game_state.resource_man.rhai.call_fn_with_options::<Dynamic>(
                scripting_rhai::rhai_call_options(&mut rhai_state),
                &mut Scope::new(),
                &script.ast,
                "selection_at_coord",
                (Dynamic::from(setup.clone()), selected),
            );

            ctx.gui.puzzle_state = Some((rhai_state.take().cast::<DataMap>(), false));

            match result {
                Ok(result) => {
                    let v = result.cast::<IdSet<ModelId>>();

                    if !v.is_empty() {
                        ctx.gui.state.research_puzzle_selections = Some((selected, v));
                    } else {
                        ctx.gui.state.research_puzzle_selections = None;
                    }

                    ctx.gui.state.selected_research_puzzle_tile = None;
                },
                Err(err) => {
                    scripting_rhai::rhai_log_err("selection_at_coord", &script.metadata.str_id, &err, None);
                    ctx.gui.state.research_puzzle_selections = None;
                },
            }
        }
    }

    if let Some((data, ..)) = &mut ctx.gui.puzzle_state
        && let Some(tiles) = data.map_coord_model_id(ctx.game_state.resource_man.registry.data_ids.research_board_tiles)
    {
        const BOARD_SIZE: Vec2 = Vec2::new(200.0, 200.0);

        let mut clicked = false;

        Pad::vertical(sizing::PADDING_MEDIUM).show(|| {
            section(|| {
                Scrollable::xy().child_size(Constraints::loose(BOARD_SIZE)).show(|| {
                    board_rect = RectRecorder::new()
                        .show(|| {
                            constrained(
                                Constraints {
                                    min: BOARD_SIZE,
                                    max: Vec2::INFINITY,
                                },
                                || {
                                    clicked = research_board_tiles(ctx.game_state, tiles.iter());
                                },
                            );
                        })
                        .into_inner();
                });
            });
        });

        if !completed
            && clicked
            && let Some(board_rect) = board_rect
        {
            let p = ctx.game_state.input_handler.main_pos.yak() - board_rect.pos() - PUZZLE_HEX_SIZE;
            let p = TileCoord::from_world_pos_with(p.unyak(), PUZZLE_HEX_GRID_LAYOUT);

            ctx.gui.state.selected_research_puzzle_tile = Some(p);
        }
    }

    board_rect
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn player_widget(ctx: &mut UiContext) {
    if let Some(research) = ctx
        .gui
        .state
        .selected_research
        .and_then(|id| ctx.game_state.resource_man.get_research(id))
    {
        let a;
        let b;
        let ab;

        {
            let filled_items = ctx
                .game_data
                .map_data
                .contains_research_id(ctx.game_state.resource_man.registry.data_ids.research_items_filled, research.id);
            let completed_puzzles = ctx
                .game_data
                .map_data
                .contains_research_id(ctx.game_state.resource_man.registry.data_ids.research_puzzle_completed, research.id);

            a = research.attached_puzzle.is_none() && filled_items;
            b = research.required_items.is_none() && completed_puzzles;
            ab = filled_items && completed_puzzles;
        }

        if a || b || ab {
            ctx.game_data.sub_map_datum(
                ctx.game_state.resource_man.registry.data_ids.research_items_filled,
                Datum::ResearchId(research.id),
            );
            ctx.game_data.sub_map_datum(
                ctx.game_state.resource_man.registry.data_ids.research_puzzle_completed,
                Datum::ResearchId(research.id),
            );
            ctx.game_data.add_map_datum(
                ctx.game_state.resource_man.registry.data_ids.unlocked_researches,
                Datum::ResearchId(research.id),
            );

            ctx.gui.state.selected_research_puzzle_tile = None;
            ctx.gui.state.research_puzzle_selections = None;
            ctx.gui.state.force_show_puzzle = true;
        }
    }

    let mut board_rect = None;

    let mut closed = false;
    if !ctx.game_state.input_handler.key_active(ActionType::Player) {
        ctx.gui.state.player_ui_state.clear();
        closed = true;
    }

    ctx.gui.state.player_ui_state = Window::new(gui_str!(ctx.game_state, player_menu_title))
        .closed(closed)
        .show_movable(ctx.gui.state.player_ui_state, || {
            col(|| {
                {
                    let mut row = list_row();
                    row.item_spacing = sizing::PADDING_MEDIUM;
                    row
                }
                .show(|| {
                    col(|| {
                        player_inventory(ctx, closed);
                    });

                    col(|| {
                        research_selection(ctx);
                    });
                });

                col(|| {
                    current_research(ctx);
                });

                row(|| {
                    col(|| {
                        board_rect = research_puzzle(ctx);
                    });

                    Pad::horizontal(sizing::PADDING_MEDIUM).show(|| {
                        col(|| {
                            if let Some(id) = ctx.gui.state.selected_research
                                && ctx
                                    .game_data
                                    .map_data
                                    .contains_research_id(ctx.game_state.resource_man.registry.data_ids.unlocked_researches, id)
                                && let Some(research) = ctx.game_state.resource_man.get_research(id)
                            {
                                divider(colors::BACKGROUND_3.yak(), sizing::DIVIER_HEIGHT, sizing::DIVIER_THICKNESS);

                                Scrollable::vertical()
                                    .child_size(Constraints::loose(Vec2::new(460.0, 130.0)))
                                    .show(|| {
                                        section(|| {
                                            Text::normal(
                                                ctx.game_state.resource_man.research_str(research.completed_description).to_string(),
                                            )
                                            .show();
                                        });
                                    });
                            }
                        });
                    });
                });
            });
        });

    if let Some((puzzle_data, dirty)) = &mut ctx.gui.puzzle_state {
        let board_tiles = puzzle_data.map_coord_model_id_mut_or_default(ctx.game_state.resource_man.registry.data_ids.research_board_tiles);
        let mut select_result = None;

        if let Some((coord, model_ids)) = &ctx.gui.state.research_puzzle_selections
            && let Some(board_rect) = board_rect
        {
            let p = (coord.to_world_pos_with(PUZZLE_HEX_GRID_LAYOUT).yak() + board_rect.pos()).round();

            reflow(Alignment::TOP_LEFT, Pivot::BOTTOM_LEFT, Dim2::ZERO, || {
                offset(Vec2::new(p.x + 20.0, p.y + 20.0), || {
                    RoundRect::new(sizing::ROUNDED_LARGE)
                        .color(colors::BACKGROUND_1.yak())
                        .show_children(|| {
                            Pad::all(sizing::PADDING_MEDIUM).show(|| {
                                Scrollable::horizontal()
                                    .child_size(Constraints::loose(Vec2::new(200.0, f32::INFINITY)))
                                    .show(|| {
                                        row(|| {
                                            for &model_id in
                                                std::iter::chain([&ctx.game_state.resource_man.registry.model_ids.puzzle_space], model_ids)
                                            {
                                                let model_id = ctx.game_state.resource_man.model_or_missing_item(model_id);

                                                let select = interactive(|| {
                                                    GameModel::new(GenericModel::Plain(model_id), PUZZLE_HEX_SIZE * 2.0).show();
                                                });

                                                if select.clicked {
                                                    select_result = Some((*coord, model_id));
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

        if let Some((selected, id)) = select_result {
            board_tiles.insert(selected, id);

            ctx.gui.state.selected_research_puzzle_tile = None;
            ctx.gui.state.research_puzzle_selections = None;
        }
    }
}
