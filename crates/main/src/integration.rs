use std::sync::Arc;

use automancy_data::{game::generic::DataMap, id::TileId};
use automancy_game::{
    actor::{FlatTiles, message::GameMsg},
    input::{ActionType, GameInputEvent},
    resources::{MutableResourceManager, RESOURCES_PATH, ResourceManager},
};
use automancy_ui::{AutomancyUiContext, state::Screen};
use winit::event::Event;

mod ui_ext {
    use automancy_data::{
        game::{
            coord::TileCoord,
            generic::{DataMap, Datum},
        },
        id::{Id, TileId},
    };
    use automancy_game::actor::{
        TileEntry,
        message::{GameMsg, PlaceTileResponse, TileMsg},
    };
    use automancy_ui::AutomancyUiContext as UiContext;
    use ractor::rpc::CallResult;

    #[cfg_attr(feature = "profile", profiling::function)]
    #[inline]
    pub fn link_tile(ctx: &mut UiContext, datum_id: Id, link_from: TileEntry, link_to: TileCoord) -> anyhow::Result<()> {
        let Ok(CallResult::Success(old)) = ctx
            .game_state
            .tokio
            .block_on(link_from.handle.call(|reply| TileMsg::GetDatum(datum_id, reply), None))
        else {
            return Ok(());
        };

        if old.is_some() {
            link_from.handle.send_message(TileMsg::RemoveDatum(datum_id))?;

            ctx.game_state.audio_man.play(ctx.game_state.resource_man.audio["click"].clone())?;
            // TODO click2
        } else {
            link_from
                .handle
                .send_message(TileMsg::SetDatum(datum_id, Datum::TileCoord(link_to)))?;

            ctx.game_state.audio_man.play(ctx.game_state.resource_man.audio["click"].clone())?;
        }

        Ok(())
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    #[inline]
    pub fn place_tile(ctx: &mut UiContext, id: TileId, coord: TileCoord) -> anyhow::Result<()> {
        let response = ctx
            .game_state
            .tokio
            .block_on(ctx.game_state.game_handle.call(
                |reply| GameMsg::PlaceTile {
                    coord,
                    tile: (id, DataMap::new()),
                    record: true,
                    reply: Some(reply),
                },
                None,
            ))?
            .unwrap();

        match response {
            PlaceTileResponse::Placed => {
                ctx.game_state.config_open_at = Some(coord);
                ctx.gui.state.last_placed_at = Some(coord);

                ctx.game_state
                    .audio_man
                    .play(ctx.game_state.resource_man.audio["tile_placement"].clone())?;
            },
            PlaceTileResponse::Removed => {
                ctx.game_state
                    .audio_man
                    .play(ctx.game_state.resource_man.audio["tile_removal"].clone())?;
            },
            _ => {},
        }

        Ok(())
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
pub fn handle_winit_event(ctx: &mut AutomancyUiContext, event: Event<()>) -> anyhow::Result<()> {
    let mut window_event = None;
    let mut device_event = None;

    match &event {
        Event::WindowEvent {
            event,
            window_id,
        } if window_id == &ctx.winit_window.id() => {
            window_event = Some(event);
        },

        Event::DeviceEvent {
            event, ..
        } => {
            device_event = Some(event);
        },

        _ => {},
    };

    if window_event.is_some() || device_event.is_some() {
        ctx.game_state.input_handler.reset();

        ctx.game_state.input_handler.update(GameInputEvent::from_winit_event(
            window_event,
            device_event,
            ctx.render.res.viewport_size_f32(),
            1.0, //TODO sensitivity option
        ));

        ctx.game_state.camera.handle_input(&ctx.game_state.input_handler);

        ctx.gui.state.input_hints.clear();

        ctx.gui.state.input_hints.push(vec![ActionType::Cancel]);
        if ctx.game_state.input_handler.key_active(ActionType::Cancel) {
            // one by one
            if ctx.gui.state.selected_tile_id.take().is_none()
                && ctx.gui.state.linking_tile.take().is_none()
                && ctx.gui.state.paste_from.take().is_none()
            {
                if ctx.gui.state.switch_screen_if(Screen::Paused, &|s| s.screen == Screen::Ingame) {
                    ctx.game_state.game_handle.send_message(GameMsg::SaveMap)?;
                } else {
                    ctx.gui.state.switch_screen_if(Screen::Ingame, &|s| s.screen == Screen::Paused);
                }
            }
        }

        ctx.gui.state.input_hints.push(vec![ActionType::Player]);

        // TODO hint this
        if (ctx.game_state.input_handler.main_pressed
            || (ctx.game_state.input_handler.key_active(ActionType::SelectMode) && ctx.game_state.input_handler.main_held))
            && ctx.gui.state.last_placed_at != Some(ctx.game_state.camera.cursor_coord)
            && let Some(id) = ctx.gui.state.selected_tile_id
        {
            ui_ext::place_tile(ctx, id, ctx.game_state.camera.cursor_coord)?;
        }

        ctx.gui.state.input_hints.push(vec![ActionType::Delete]);
        if ctx.game_state.input_handler.key_active(ActionType::Delete) {
            ui_ext::place_tile(ctx, TileId::none(), ctx.game_state.camera.cursor_coord)?;
        }

        if !ctx.game_state.input_handler.key_active(ActionType::SelectMode) {
            // TODO hint this
            if ctx.game_state.input_handler.alternate_pressed {
                if let Some((_coord, datum_id, link_from)) = ctx.gui.state.linking_tile.clone() {
                    ui_ext::link_tile(ctx, datum_id, link_from, ctx.game_state.camera.cursor_coord)?;
                } else if ctx.game_state.config_open_at == Some(ctx.game_state.camera.cursor_coord) {
                    ctx.game_state.config_open_at = None;
                } else {
                    ctx.game_state.config_open_at = Some(ctx.game_state.camera.cursor_coord);
                }
            }
        }

        ctx.gui.state.input_hints.push(vec![ActionType::SelectMode]);
        if ctx.game_state.input_handler.key_active(ActionType::SelectMode) && ctx.gui.state.screen == Screen::Ingame {
            if ctx.gui.state.paste_from.is_none() {
                ctx.gui.state.grouped_tiles.insert(ctx.game_state.camera.cursor_coord);
            }
        } else {
            ctx.gui.state.grouped_tiles.clear();
        }

        if ctx.game_state.input_handler.key_active(ActionType::HotkeyActive) {
            ctx.gui.state.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Undo]);
            ctx.gui.state.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Redo]);

            if ctx.game_state.input_handler.key_active(ActionType::Undo) {
                ctx.game_state.game_handle.send_message(GameMsg::Undo)?;
            }
            if ctx.game_state.input_handler.key_active(ActionType::Redo) {
                //TODO game_state.game_handle.send_message(GameMsg::Redo)?;
            }

            if ctx.gui.state.paste_from.is_none() && !ctx.gui.state.grouped_tiles.is_empty() {
                ctx.gui.state.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Cut]);
                ctx.gui.state.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Copy]);

                if ctx.game_state.input_handler.key_active(ActionType::Cut) || ctx.game_state.input_handler.key_active(ActionType::Copy) {
                    ctx.gui.state.paste_from = Some(ctx.game_state.camera.cursor_coord);
                    ctx.game_state.audio_man.play(ctx.game_state.resource_man.audio["click"].clone())?;

                    let coords = Vec::from_iter(std::mem::take(&mut ctx.gui.state.grouped_tiles));

                    if ctx.game_state.input_handler.key_active(ActionType::Cut) {
                        ctx.gui.state.paste_content = ctx
                            .game_state
                            .tokio
                            .block_on(ctx.game_state.game_handle.call(
                                |reply| {
                                    GameMsg::PlaceTiles {
                                        tiles: coords
                                            .into_iter()
                                            .map(|coord| (coord, (TileId::none(), DataMap::new())))
                                            .collect::<FlatTiles>(),
                                        replace: true,
                                        record: true,
                                        reply: Some(reply),
                                    }
                                },
                                None,
                            ))?
                            .unwrap();
                    } else {
                        ctx.gui.state.paste_content = ctx
                            .game_state
                            .tokio
                            .block_on(ctx.game_state.game_handle.call(|reply| GameMsg::GetTilesFlat(coords, reply), None))?
                            .unwrap();
                    }
                }
            }

            if let Some(start) = ctx.gui.state.paste_from {
                ctx.gui.state.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Paste]);

                if ctx.game_state.input_handler.key_active(ActionType::Paste) {
                    let direction = ctx.game_state.camera.cursor_coord - start;

                    ctx.game_state.game_handle.send_message(GameMsg::PlaceTiles {
                        tiles: ctx
                            .gui
                            .state
                            .paste_content
                            .clone()
                            .into_iter()
                            .map(|(coord, tile)| (coord + direction, tile))
                            .collect::<FlatTiles>(),
                        replace: false,
                        record: true,
                        reply: None,
                    })?;

                    ctx.game_state.audio_man.play(ctx.game_state.resource_man.audio["click"].clone())?;
                    // TODO click2
                }
            }
        } else {
            ctx.gui.state.input_hints.push(vec![ActionType::HotkeyActive]);
        }

        if ctx.game_state.input_handler.key_active(ActionType::Fullscreen) {
            ctx.game_state.options.graphics.fullscreen = !ctx.game_state.options.graphics.fullscreen;
            ctx.game_state.options.synced = false
        }

        ctx.gui.state.input_hints.push(vec![ActionType::ToggleGui]);

        ctx.render.screenshotting = ctx.game_state.input_handler.key_active(ActionType::Screenshot);
    }

    Ok(())
}

#[cfg_attr(feature = "profile", profiling::function)]
pub fn load_resources(lang: &str) -> Arc<ResourceManager> {
    let mut resource_man = MutableResourceManager::new();

    for dir in std::fs::read_dir(RESOURCES_PATH)
        .expect("the resources folder needs to exist and be readable")
        .flatten()
        .map(|v| v.path())
        .filter(|v| v.is_dir())
    {
        let namespace = dir.file_name().unwrap().to_str().unwrap().trim();
        log::info!("Loading namespace {namespace}...");

        #[cfg(miri)]
        if namespace != "core" {
            log::debug!("Only namespace core is loaded when using miri.");
            continue;
        }

        resource_man.load_model_files(&dir, namespace);
        #[cfg(not(miri))]
        resource_man.load_audio_files(&dir);
        resource_man.load_tile_files(&dir, namespace);
        resource_man.load_item_files(&dir, namespace);
        resource_man.load_tag_files(&dir, namespace);
        resource_man.load_category_files(&dir, namespace);
        resource_man.load_recipe_files(&dir, namespace);
        resource_man.load_translate_files(&dir, namespace, lang);
        resource_man.load_shader_files(&dir);
        #[cfg(not(miri))]
        resource_man.load_font_files(&dir);
        resource_man.load_script_files(&dir, namespace);
        resource_man.load_research_files(&dir, namespace);

        log::info!("Loaded namespace {namespace}!");
    }

    MutableResourceManager::compile(resource_man)
}
