use std::mem;

use egui::scroll_area::ScrollBarVisibility;
use egui::{pos2, vec2, Button, Frame, Pos2, Rect, ScrollArea, Sense, Window};
use rhai::Dynamic;

use automancy_defs::coord::TileCoord;
use automancy_defs::glam::{dvec3, vec3, Vec2};
use automancy_defs::graph::visit::Topo;
use automancy_defs::hexx::{HexLayout, HexOrientation};
use automancy_defs::id::Id;
use automancy_defs::math;
use automancy_defs::math::Float;
use automancy_defs::rendering::InstanceData;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::types::function::RhaiDataMap;
use automancy_resources::types::IconMode;
use automancy_resources::{rhai_call_options, rhai_log_err};

use crate::gui::item::draw_item;
use crate::gui::{
    take_item_animation, GameEguiCallback, MEDIUM_ICON_SIZE, SMALLISH_ICON_SIZE, SMALL_ICON_SIZE,
};
use crate::util::is_research_unlocked;
use crate::GameState;

const PUZZLE_HEX_GRID_LAYOUT: HexLayout = HexLayout {
    orientation: HexOrientation::Pointy,
    origin: Vec2::ZERO,
    hex_size: automancy_defs::glam::vec2(SMALLISH_ICON_SIZE, SMALLISH_ICON_SIZE),
    invert_x: false,
    invert_y: true,
};

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
                            .max_height(600.0)
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
                            ui.vertical(|ui| {
                                ui.vertical(|ui| {
                                    ui.heading(state.resource_man.research_str(&research.name));
                                    ui.label(state.resource_man.research_str(&research.description));

                                    let mut already_unlocked = false;
                                    if let Some(Data::SetId(unlocked)) = game_data
                                        .get(&state.resource_man.registry.data_ids.unlocked_researches)
                                    {
                                        already_unlocked = unlocked.contains(&research.id)
                                    }

                                    if !already_unlocked {
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
                                    } else {
                                        if research.required_items.is_some() {
                                            game_data
                                                .get(
                                                    &state
                                                        .resource_man
                                                        .registry
                                                        .data_ids
                                                        .research_items_filled,
                                                )
                                                .and_then(|filled_items| match filled_items {
                                                    Data::SetId(set) => Some(set.contains(&research.id)),
                                                    _ => None,
                                                })
                                                .unwrap_or(false)
                                        } else {
                                            true
                                        }
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

                                        let puzzle_state = state.puzzle_state.get_or_insert_with(|| {
                                            let data = RhaiDataMap::default();
                                            let mut rhai_state = Dynamic::from(data);

                                            let result = state.resource_man.engine.call_fn_with_options::<()>(
                                                rhai_call_options(&mut rhai_state),
                                                &mut scope,
                                                ast,
                                                "pre_setup",
                                                (
                                                    Dynamic::from(setup.clone()),
                                                ),
                                            );


                                            match result {
                                                Err(err) => rhai_log_err(function_id, &err),
                                                _ => {}
                                            }

                                            (rhai_state.take().cast::<RhaiDataMap>(), true)
                                        });

                                        if puzzle_state.1 {
                                            let mut rhai_state = Dynamic::from(mem::take(&mut puzzle_state.0));

                                            let result =
                                                state.resource_man.engine.call_fn_with_options::<bool>(
                                                    rhai_call_options(&mut rhai_state),
                                                    &mut scope,
                                                    ast,
                                                    "evaluate",
                                                    (
                                                        Dynamic::from(setup.clone()),
                                                    ),
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

                                        if let Some(selected) =
                                            state.gui_state.selected_research_puzzle_tile
                                        {
                                            let mut rhai_state = Dynamic::from(mem::take(&mut puzzle_state.0));

                                            let result =
                                                state.resource_man.engine.call_fn_with_options::<Dynamic>(
                                                    rhai_call_options(&mut rhai_state),
                                                    &mut scope,
                                                    ast,
                                                    "selection_at_coord",
                                                    (
                                                        Dynamic::from(setup.clone()),
                                                        selected,
                                                    ),
                                                );


                                            state.puzzle_state =
                                                Some((rhai_state.take().cast::<RhaiDataMap>(), false));

                                            match result {
                                                Ok(result) => {
                                                    if let Some(vec) = result.try_cast::<Vec<Id>>() {
                                                        if !vec.is_empty() {
                                                            state.gui_state.research_puzzle_selections =
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
                                            ScrollArea::both()
                                                .id_source("research_puzzle")
                                                .drag_to_scroll(true)
                                                .max_width(300.0)
                                                .max_height(300.0)
                                                .show(ui, |ui| {
                                                    if let Some(Data::TileMap(tiles)) = data
                                                        .get_mut(state.resource_man.registry.data_ids.tiles)
                                                    {
                                                        let mut min = Pos2::new(Float::INFINITY, Float::INFINITY);
                                                        let r = ui.group(|ui| {
                                                            let cursor_pos = ui.cursor().min + vec2(PUZZLE_HEX_GRID_LAYOUT.hex_size.x / 2.0, 0.0);

                                                            for (coord, id) in tiles.iter() {
                                                                let [x, y] = PUZZLE_HEX_GRID_LAYOUT
                                                                    .hex_to_world_pos(**coord)
                                                                    .to_array();
                                                                let pos = pos2((cursor_pos.x + x / 2.0).round(), (cursor_pos.y + y / 2.0).round());
                                                                min = min.min(pos);

                                                                let rect = Rect::from_min_size(
                                                                    pos,
                                                                    vec2(
                                                                        PUZZLE_HEX_GRID_LAYOUT.hex_size.x,
                                                                        PUZZLE_HEX_GRID_LAYOUT.hex_size.y,
                                                                    ),
                                                                );

                                                                ui.allocate_rect(rect, Sense::hover());

                                                                ui.painter().add(
                                                                    egui_wgpu::Callback::new_paint_callback(
                                                                        rect,
                                                                        GameEguiCallback::new(
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
                                                                            rect,
                                                                            ui.ctx().screen_rect(),
                                                                        ),
                                                                    ),
                                                                );
                                                            }
                                                        });

                                                        let mut select_result = None;

                                                        if let Some((selected, ids)) =
                                                            &state.gui_state.research_puzzle_selections
                                                        {
                                                            Frame::popup(ui.style()).show(ui, |ui| {
                                                                ScrollArea::horizontal()
                                                                    .id_source("research_puzzle_selections") // TODO how do i make this, like, float??
                                                                    .scroll_bar_visibility(
                                                                        ScrollBarVisibility::AlwaysVisible,
                                                                    )
                                                                    .drag_to_scroll(true)
                                                                    .auto_shrink(true)
                                                                    .max_width(100.0)
                                                                    .show(ui, |ui| {
                                                                        ui.horizontal_top(|ui| {
                                                                            let (rect, response) =
                                                                                ui.allocate_exact_size(vec2(SMALLISH_ICON_SIZE, SMALLISH_ICON_SIZE), Sense::click());

                                                                            if response.clicked() {
                                                                                select_result = Some((*selected, state.resource_man.registry.model_ids.puzzle_space));
                                                                                *dirty = true;
                                                                            }

                                                                            ui.painter().add(
                                                                                egui_wgpu::Callback::new_paint_callback(
                                                                                    rect,
                                                                                    GameEguiCallback::new(
                                                                                        InstanceData::default()
                                                                                            .with_world_matrix(
                                                                                                math::view(dvec3(
                                                                                                    0.0, 0.0, 1.0,
                                                                                                )).as_mat4(),
                                                                                            ),
                                                                                        state.resource_man.registry.model_ids.puzzle_space,
                                                                                        rect,
                                                                                        ui.ctx().screen_rect(),
                                                                                    ),
                                                                                ),
                                                                            );

                                                                            for id in ids {
                                                                                let (rect, response) =
                                                                                    ui.allocate_exact_size(vec2(SMALLISH_ICON_SIZE, SMALLISH_ICON_SIZE), Sense::click());

                                                                                if response.clicked() {
                                                                                    select_result = Some((*selected, *id));
                                                                                    *dirty = true;
                                                                                }

                                                                                ui.painter().add(
                                                                                    egui_wgpu::Callback::new_paint_callback(
                                                                                        rect,
                                                                                        GameEguiCallback::new(
                                                                                            InstanceData::default()
                                                                                                .with_world_matrix(
                                                                                                    math::view(dvec3(
                                                                                                        0.0, 0.0, 1.0,
                                                                                                    )).as_mat4(),
                                                                                                ),
                                                                                            state.resource_man.get_item_model(
                                                                                                state
                                                                                                    .resource_man
                                                                                                    .registry
                                                                                                    .items[id]
                                                                                                    .model,
                                                                                            ),
                                                                                            rect,
                                                                                            ui.ctx().screen_rect(),
                                                                                        ),
                                                                                    ),
                                                                                );
                                                                            }
                                                                        });
                                                                    });
                                                            });
                                                        }

                                                        if let Some((selected, id)) = select_result {
                                                            tiles.insert(selected, id);

                                                            state
                                                                .gui_state
                                                                .selected_research_puzzle_tile = None;
                                                            state.gui_state.research_puzzle_selections = None;
                                                        }

                                                        if r.response
                                                            .interact(Sense::click())
                                                            .clicked() {
                                                            if let Some(pos) = ui.ctx().pointer_interact_pos() {
                                                                let p = (pos - min) * 2.0 - vec2(PUZZLE_HEX_GRID_LAYOUT.hex_size.x * 2.0, PUZZLE_HEX_GRID_LAYOUT.hex_size.y);

                                                                let p = TileCoord::from(PUZZLE_HEX_GRID_LAYOUT
                                                                    .world_pos_to_hex(automancy_defs::glam::vec2(p.x, p.y)));


                                                                state.gui_state.selected_research_puzzle_tile = Some(p);
                                                            }
                                                        }
                                                    }
                                                });
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

                                state.gui_state.selected_research_puzzle_tile = None;
                                state.gui_state.research_puzzle_selections = None;
                            }

                            if let Some(Data::SetId(set)) =
                                game_data.get(&state.resource_man.registry.data_ids.unlocked_researches)
                            {
                                let research = &state
                                    .gui_state
                                    .selected_research
                                    .and_then(|id| state.resource_man.get_research(id))
                                    .unwrap();

                                if set.contains(&research.id) {
                                    ui.label(
                                        state
                                            .resource_man
                                            .research_str(&research.completed_description),
                                    );
                                }
                            }
                        }
                    });
                });
            });
        });
}
