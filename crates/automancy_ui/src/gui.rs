use crate::*;

pub mod debug_widget;
pub mod error_widget;
pub mod info_widget;
pub mod main_menu;
pub mod maps_menu;
pub mod options_menu;
pub mod pause_menu;
pub mod player_widget;
pub mod tile_config_widget;
pub mod tile_selection_widget;

#[cfg_attr(feature = "profile", profiling::function)]
pub fn layout_ui(ctx: &mut UiContext) {
    debug_widget::debug_widget(ctx);

    match ctx.gui.state.popup.clone() {
        PopupState::None => match &ctx.gui.state.screen {
            Screen::Ingame => {
                info_widget::info_widget(ctx);

                if matches!(ctx.game_data.loaded_map.read_latest(), Some((GameMapId::SaveFile(..), ..)))
                    && !ctx.game_state.input_handler.key_active(ActionType::ToggleGui)
                {
                    {
                        let selection_result = tile_selection_widget::tile_selection_widget(ctx);

                        if selection_result.clicked
                            && let Some(id) = selection_result.hovering_tile
                        {
                            ctx.gui.state.last_placed_at = None;

                            if ctx.gui.state.selected_tile_id == Some(id) {
                                ctx.gui.state.selected_tile_id = None;
                            } else {
                                ctx.gui.state.selected_tile_id = Some(id);
                            }
                        }
                    }

                    player_widget::player_widget(ctx);

                    tile_config_widget::tile_config_widget(ctx);
                }
            },
            Screen::MainMenu => main_menu::main_menu(ctx),
            Screen::MapLoad => {
                maps_menu::map_menu(ctx);
            },
            Screen::Options(menu) => {
                options_menu::options_menu(ctx, *menu);
            },
            Screen::Paused => {
                pause_menu::pause_menu(ctx);
            },
        },
        PopupState::MapCreate => maps_menu::map_create_popup(ctx),
        PopupState::MapDeleteConfirmation(map_name) => {
            maps_menu::map_delete_popup(ctx, &map_name);
        },
        PopupState::InvalidName => {
            maps_menu::map_name_invalid_popup(ctx);
        },
    }

    error_widget::error_widget(ctx);

    render_info_tip();
    render_item_animations();

    {
        for coord in &ctx.gui.state.grouped_tiles {
            ctx.render
                .game_renderer
                .tile_tints
                .insert(*coord, colors::OUTPUT.with_alpha_packed(102));
        }

        ctx.render
            .game_renderer
            .tile_tints
            .insert(ctx.game_state.camera.cursor_coord, colors::IMPORTANT.with_alpha_packed(64));
    }

    {
        {
            ctx.render_state
                .remove_overlay_model(ctx.game_state.resource_man.registry.render_ids.selected_tile);
            ctx.render_state.flush_overlay_model();
            ctx.render_state.add_overlay_model(
                ctx.game_state,
                ctx.game_state.camera.cursor_coord,
                if let Some(tile_id) = ctx.gui.state.selected_tile_id {
                    GenericModel::Tile(tile_id)
                } else {
                    GenericModel::None
                },
                ctx.game_state.resource_man.registry.render_ids.selected_tile,
                &mut DataMap::new(),
                GameDrawInstance {
                    alpha: 0.75,
                    ..Default::default()
                },
            );
        }

        if ctx.gui.state.linking_tile != ctx.gui.prev_state.linking_tile {
            if let Some((coord, ..)) = &ctx.gui.state.linking_tile {
                ctx.render_state.add_overlay_model(
                    ctx.game_state,
                    *coord,
                    GenericModel::Plain(ctx.game_state.resource_man.registry.model_ids.cube1x1),
                    ctx.game_state.resource_man.registry.render_ids.linking_line,
                    &mut DataMap::new(),
                    GameDrawInstance {
                        color_offset: colors::IMPORTANT.packed,
                        ..Default::default()
                    },
                );
            } else {
                ctx.render_state
                    .remove_overlay_model(ctx.game_state.resource_man.registry.render_ids.linking_line);
                ctx.render_state.flush_overlay_model();
            }
        }

        if let Some((coord, ..)) = &ctx.gui.state.linking_tile {
            ctx.render_state.set_overlay_matrix(
                *coord,
                ctx.game_state.resource_man.registry.render_ids.linking_line,
                (
                    Some(rendering::util::make_line(
                        coord.to_world_pos(),
                        ctx.game_state.camera.cursor_world_pos.xy(),
                        rendering::view::WORLD_PLANE_Z,
                    )),
                    None,
                ),
            );
        }

        if ctx.gui.state.paste_from != ctx.gui.prev_state.paste_from {
            if let Some(start) = ctx.gui.state.paste_from {
                ctx.render_state.add_overlay_model(
                    ctx.game_state,
                    start,
                    GenericModel::Plain(ctx.game_state.resource_man.registry.model_ids.cube1x1),
                    ctx.game_state.resource_man.registry.render_ids.pasting_line,
                    &mut DataMap::new(),
                    GameDrawInstance {
                        color_offset: colors::INPUT.packed,
                        ..Default::default()
                    },
                );

                for (&coord, (tile_id, data_map)) in &mut ctx.gui.state.paste_content {
                    ctx.render_state.add_overlay_model(
                        ctx.game_state,
                        coord,
                        GenericModel::Tile(*tile_id),
                        ctx.game_state.resource_man.registry.render_ids.pasting_content,
                        data_map,
                        GameDrawInstance {
                            alpha: 0.75,
                            ..Default::default()
                        },
                    );
                }
            } else {
                ctx.render_state
                    .remove_overlay_model(ctx.game_state.resource_man.registry.render_ids.pasting_line);
                ctx.render_state
                    .remove_overlay_model(ctx.game_state.resource_man.registry.render_ids.pasting_content);
                ctx.render_state.flush_overlay_model();
            }
        }

        if let Some(start) = ctx.gui.state.paste_from {
            ctx.render_state.set_overlay_matrix(
                start,
                ctx.game_state.resource_man.registry.render_ids.pasting_line,
                (
                    None,
                    Some(rendering::util::make_line(
                        start.to_world_pos(),
                        ctx.game_state.camera.cursor_coord.to_world_pos(),
                        rendering::view::WORLD_PLANE_Z,
                    )),
                ),
            );

            let diff = ctx.game_state.camera.cursor_coord - start;
            let world_matrix = {
                let p = diff.to_world_pos();

                math::Matrix4::translation_3d(math::Vec3::new(p.x, p.y, 0.0))
            };

            for (&coord, ..) in &mut ctx.gui.state.paste_content {
                ctx.render_state.set_overlay_matrix(
                    coord,
                    ctx.game_state.resource_man.registry.render_ids.pasting_content,
                    (None, Some(world_matrix)),
                );
            }
        }
    }
}
