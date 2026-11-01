use automancy_data::game::generic::{DatumChange, DatumType};
use automancy_game::script::UiElement;

use crate::*;

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn add_direction(current: &mut Option<TileCoord>, dir: TileCoord) {
    let symbol = match dir {
        TileCoord::TOP_RIGHT => "\u{f46c}",
        TileCoord::RIGHT => "\u{f432}",
        TileCoord::BOTTOM_RIGHT => "\u{f43e}",
        TileCoord::BOTTOM_LEFT => "\u{f424}",
        TileCoord::LEFT => "\u{f434}",
        TileCoord::TOP_LEFT => "\u{f45c}",
        _ => panic!("dir should be one of the 6 hexagonal unit directions"),
    };

    let res = selectable_symbol_button(symbol, colors::TEXT_ACTIVE.yak(), *current == Some(dir));

    if res.clicked {
        *current = Some(dir)
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
fn takeable_items(ctx: &mut UiContext, buffer: &mut Inventory, buffer_id: Id, tile_entity: ActorRef<TileMsg>) -> bool {
    let mut dirty = false;

    for (id, amount) in buffer.clone() {
        let mut rect = None;

        let interact = interactive(|| {
            rect = RectRecorder::new()
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
        });

        if interact.clicked {
            let amount = buffer.take(id, amount);

            if amount > 0 {
                dirty = true;

                ctx.game_data.add_map_datum(
                    ctx.game_state.resource_man.registry.data_ids.player_inventory,
                    Datum::Items(Inventory::from_iter([ItemStack {
                        id,
                        amount,
                    }])),
                );

                if let Some(rect) = rect {
                    item_animation_src(
                        ctx.game_state.resource_man.registry.render_ids.player_inventory_animation,
                        id,
                        Rect::from_pos_size(rect.pos(), Vec2::splat(sizing::MEDIUM_ICON_SIZE)),
                        const { Duration::from_millis(300) },
                    );
                }
            }
        }
    }

    if dirty {
        tile_entity
            .send_message(TileMsg::ChangeData(DatumChange::Set(buffer_id, Datum::Items(buffer.clone()))))
            .unwrap();
    }

    dirty
}

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn rhai_ui(ctx: &mut UiContext, tile_entity: ActorRef<TileMsg>, data: &mut DataMap, ui: &UiElement, dirty: &mut bool) {
    let mut handle_change = |data: &mut DataMap, change: DatumChange| {
        data.handle_change(change.clone());
        tile_entity.send_message(TileMsg::ChangeData(change)).unwrap();
        *dirty = true;
    };

    match ui {
        &UiElement::Label {
            id,
        } => {
            Text::normal(ctx.game_state.resource_man.gui_str(GuiTranslateId(id))).show();
        },
        &UiElement::InfoTip {
            id,
        } => {
            info_tip(ctx.game_state.resource_man.gui_str(GuiTranslateId(id)));
        },
        &UiElement::LabelAmount {
            amount,
        } => {
            Text::normal(amount.to_string()).show();
        },
        &UiElement::InputAmount {
            id,
            max,
        } => {
            let current_amount = data.int(id).copied().unwrap_or_default();

            if let Some(new_amount) = NumberInput::new_with_range(current_amount, 0..=max).show().value {
                handle_change(data, DatumChange::Set(id, Datum::Int(new_amount)));
            }
        },
        &UiElement::SliderAmount {
            id,
            max,
        } => {
            let current_amount = data.int(id).copied().unwrap_or_default();

            if let Some(new_amount) = slider_with_input(current_amount, 0..=max, None) {
                handle_change(data, DatumChange::Set(id, Datum::Int(new_amount)));
            }
        },
        &UiElement::HexDirInput {
            id,
        } => {
            let current_dir = data.coord(id).copied();
            let mut new_dir = current_dir;

            col_cross_center(|| {
                constrained(Constraints::loose(Vec2::new(86.0, f32::INFINITY)), || {
                    Pad::all(sizing::PADDING_MEDIUM).show(|| {
                        list_col().item_spacing(sizing::PADDING_XSMALL).show(|| {
                            row_spaced_evenly(|| {
                                add_direction(&mut new_dir, TileCoord::TOP_LEFT);
                                add_direction(&mut new_dir, TileCoord::TOP_RIGHT);
                            });

                            row_spaced_evenly(|| {
                                add_direction(&mut new_dir, TileCoord::LEFT);
                                if symbol_button("\u{f467}", colors::IMPORTANT.yak()).clicked {
                                    new_dir = None;
                                }
                                add_direction(&mut new_dir, TileCoord::RIGHT);
                            });

                            row_spaced_evenly(|| {
                                add_direction(&mut new_dir, TileCoord::BOTTOM_LEFT);
                                add_direction(&mut new_dir, TileCoord::BOTTOM_RIGHT);
                            });
                        });
                    });
                });
            });

            if new_dir != current_dir {
                if let Some(coord) = new_dir {
                    handle_change(data, DatumChange::Set(id, Datum::TileCoord(coord)));
                } else {
                    handle_change(data, DatumChange::Remove(id, DatumType::TileCoord));
                }
            }
        },
        UiElement::SelectableItems {
            data_id,
            hint_id,
            ids,
        } => {
            let hint = ctx.game_state.resource_man.gui_str(GuiTranslateId(*hint_id));

            let old_id = data.item_id(*data_id).copied();
            let mut current_id = old_id;
            searchable_ids(
                &mut current_id,
                ids.iter(),
                ctx.gui.state.text_field.get(TextField::Filter),
                Some(hint.into()),
                |id| draw_item(&ctx.game_state.resource_man, id, sizing::SMALL_ICON_SIZE),
                |id| ctx.game_state.resource_man.item_name(id),
            );

            if current_id != old_id {
                handle_change(data, DatumChange::Set(*data_id, Datum::ItemId(current_id.unwrap_or_default())));
            }
        },
        UiElement::SelectableRecipes {
            data_id,
            hint_id,
            ids,
        } => {
            let hint = ctx.game_state.resource_man.gui_str(GuiTranslateId(*hint_id));

            let old_id = data.recipe_id(*data_id).copied();
            let mut current_id = old_id;
            searchable_ids(
                &mut current_id,
                ids.iter(),
                ctx.gui.state.text_field.get(TextField::Filter),
                Some(hint.into()),
                |id| draw_recipe(&ctx.game_state.resource_man, id, sizing::SMALL_ICON_SIZE),
                |id| ctx.game_state.resource_man.recipe_name(id),
            );

            if current_id != old_id {
                handle_change(data, DatumChange::Set(*data_id, Datum::RecipeId(current_id.unwrap_or_default())));
            }

            if let Some(recipe_id) = current_id {
                draw_recipe_info(ctx.game_state, recipe_id, sizing::SMALL_ICON_SIZE);
            }
        },
        &UiElement::Items {
            id,
            empty_text,
        } => {
            col(|| {
                let stacks = data.items_mut_or_default(id);

                if !stacks.is_empty() {
                    if takeable_items(ctx, stacks, id, tile_entity.clone()) {
                        *dirty = true;
                    }
                } else {
                    Text::normal(ctx.game_state.resource_man.gui_str(GuiTranslateId(empty_text))).show();
                }
            });
        },
        &UiElement::Linkage {
            id,
            coord,
            button_text,
        } => {
            if button(ctx.game_state.resource_man.gui_str(GuiTranslateId(button_text))).clicked
                && let Some(tile) = ctx
                    .game_state
                    .tokio
                    .block_on(ctx.game_state.game_handle.call(|reply| GameMsg::GetTile(coord, reply), None))
                    .unwrap()
                    .unwrap()
            {
                ctx.gui.state.linking_tile = Some((coord, id, tile));
            };
        },
        UiElement::Row {
            e,
        } => {
            row(|| {
                for ui in e {
                    rhai_ui(ctx, tile_entity.clone(), data, ui, dirty);
                }
            });
        },
        UiElement::CenterRow {
            e,
        } => {
            row_cross_center(|| {
                for ui in e {
                    rhai_ui(ctx, tile_entity.clone(), data, ui, dirty);
                }
            });
        },
        UiElement::Col {
            e,
        } => {
            list_col().item_spacing(sizing::PADDING_XSMALL).show(|| {
                for ui in e {
                    rhai_ui(ctx, tile_entity.clone(), data, ui, dirty);
                }
            });
        },
    }
}
