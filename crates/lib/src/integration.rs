use std::time::Instant;

use automancy_data::{
    game::generic::DataMap,
    id::TileId,
    rendering::{colors, colors::ColorExt},
};
use automancy_game::{
    actor::{FlatTiles, message::GameMsg},
    input,
    input::handler::ActionType,
    state::{AutomancyGameState, ui, ui::TextField},
};
use automancy_rendering::{renderer::AutomancyRenderState, window_util};
use winit::{
    event::{Event, WindowEvent},
    event_loop::ActiveEventLoop,
};

use crate::render::AutomancyRendering;

pub mod util {
    use std::{fs, time::SystemTime};

    use automancy_data::{
        game::{
            coord::TileCoord,
            generic::{DataMap, Datum},
        },
        id::{Id, TileId},
    };
    use automancy_game::{
        actor::{
            TileEntry, map,
            map::GameMapId,
            message::{GameMsg, PlaceTileResponse, TileMsg},
        },
        state::AutomancyGameState,
    };
    use automancy_rendering::renderer::AutomancyRenderState;
    use ractor::{ActorRef, rpc::CallResult};
    use tokio::task::JoinHandle;
    use winit::event_loop::ActiveEventLoop;

    use crate::render::AutomancyRendering;

    /// Refreshes the list of maps on the filesystem. Should be done every time the list of maps could have changed (on map creation/delete and on game load).
    pub fn refresh_maps(state: &mut AutomancyGameState) {
        fs::create_dir_all(map::MAP_PATH).unwrap();

        state.loop_store.map_infos_cache = fs::read_dir(map::MAP_PATH)
            .expect("map folder needs to exist and be readable")
            .flatten()
            .map(|f| f.file_name().to_str().unwrap().to_string())
            .filter(|f| !f.starts_with('.'))
            .flat_map(|name| {
                map::serialize::read_map_data(
                    &state.resource_man,
                    &GameMapId::SaveFile(name.clone()),
                )
                .ok()
                .zip(Some(name))
            })
            .collect::<Vec<_>>();

        state
            .loop_store
            .map_infos_cache
            .sort_by(|a, b| a.1.cmp(&b.1));
        state.loop_store.map_infos_cache.sort_by(|a, b| {
            a.0.1
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .cmp(&b.0.1.unwrap_or(SystemTime::UNIX_EPOCH))
        });
        state.loop_store.map_infos_cache.reverse();
    }

    pub async fn shutdown_graceful(
        game: &ActorRef<GameMsg>,
        game_handle: &mut Option<JoinHandle<()>>,
        event_loop: &ActiveEventLoop,
    ) -> anyhow::Result<()> {
        game.call(GameMsg::SaveAndUnload, None)
            .await
            .expect("the game needs to save the map on exit");
        game.stop(Some("game closed".to_string()));
        game_handle
            .take()
            .expect("game handle needs to be set")
            .await?;

        event_loop.exit();

        log::info!("Shut down gracefully.");

        Ok(())
    }

    pub(super) fn render(
        closed: &mut bool,
        event_loop: &ActiveEventLoop,
        game_state: &mut AutomancyGameState,
        render_state: &mut AutomancyRenderState,
        render: &mut AutomancyRendering,
    ) -> anyhow::Result<()> {
        {
            if let Ok(mut cache) = game_state
                .loop_store
                .config_open_cache
                .clone()
                .try_lock_owned()
            {
                let game = game_state.game.clone();
                let config_open_at = game_state.ui_state.config_open_at;

                game_state.tokio.spawn(async move {
                    let Some(config_open_at) = config_open_at else {
                        *cache = None;

                        return;
                    };

                    let Ok(CallResult::Success(tile)) = game
                        .call(|reply| GameMsg::GetTile(config_open_at, reply), None)
                        .await
                    else {
                        return;
                    };

                    *cache = tile.map(|v| v.handle);
                });
            }

            if let Ok(mut cache) = game_state
                .loop_store
                .pointing_cache
                .clone()
                .try_lock_owned()
            {
                let game = game_state.game.clone();
                let pointing_at = game_state.camera.pointing_at;

                game_state.tokio.spawn(async move {
                    let Ok(CallResult::Success(tile)) = game
                        .call(|reply| GameMsg::GetTile(pointing_at, reply), None)
                        .await
                    else {
                        return;
                    };

                    *cache = tile;
                });
            }
        }

        {
            /*
            {
                state.gui.as_mut().unwrap().yak.start();

                gui::render_ui(state, &mut result, event_loop);

                state.gui.as_mut().unwrap().yak.finish();
            }
             */

            match render.renderer.render(
                game_state,
                render_state,
                &mut render.res,
                render.screenshotting,
            ) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {
                    render.res.resize(render.res.window.inner_size());
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    *closed = true;
                    return game_state.tokio.block_on(shutdown_graceful(
                        &game_state.game,
                        &mut game_state.game_handle,
                        event_loop,
                    ));
                }
                Err(e) => log::error!("{e:?}"),
            }
        }

        Ok(())
    }

    pub(super) fn link_tile(
        state: &mut AutomancyGameState,
        id: Id,
        link_from: TileEntry,
        link_to: TileCoord,
    ) -> anyhow::Result<()> {
        let Ok(CallResult::Success(old)) = state.tokio.block_on(
            link_from
                .handle
                .call(|reply| TileMsg::GetDatum(id, reply), None),
        ) else {
            return Ok(());
        };

        if old.is_some() {
            link_from.handle.send_message(TileMsg::RemoveDatum(id))?;

            state
                .audio_man
                .play(state.resource_man.audio["click"].clone())?;
            // TODO click2
        } else {
            link_from
                .handle
                .send_message(TileMsg::SetDatum(id, Datum::Coord(link_to)))?;

            state
                .audio_man
                .play(state.resource_man.audio["click"].clone())?;
        }

        Ok(())
    }

    pub(super) fn place_tile(
        id: TileId,
        coord: TileCoord,
        state: &mut AutomancyGameState,
    ) -> anyhow::Result<()> {
        let response = state
            .tokio
            .block_on(state.game.call(
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
                state.ui_state.config_open_at = Some(coord);
                state.ui_state.already_placed_at = Some(coord);

                state
                    .audio_man
                    .play(state.resource_man.audio["tile_placement"].clone())?;
            }
            PlaceTileResponse::Removed => {
                state
                    .audio_man
                    .play(state.resource_man.audio["tile_removal"].clone())?;
            }
            _ => {}
        }

        Ok(())
    }
}

/// Triggers every time the event loop is run once.
pub fn on_event(
    closed: &mut bool,
    event_loop: &ActiveEventLoop,
    game_state: &mut AutomancyGameState,
    render_state: &mut AutomancyRenderState,
    render: &mut AutomancyRendering,
    event: Event<()>,
) -> anyhow::Result<()> {
    let mut window_event = None;
    let mut device_event = None;

    match &event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            // game shutdown
            *closed = true;
            return game_state.tokio.block_on(util::shutdown_graceful(
                &game_state.game,
                &mut game_state.game_handle,
                event_loop,
            ));
        }
        Event::WindowEvent { event, window_id } if window_id == &render.res.window.id() => {
            match event {
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();

                    game_state.loop_store.elapsed =
                        now - game_state.loop_store.frame_start.take().unwrap();

                    game_state.camera.update_pointing_at(
                        window_util::window_size(&render.res.window),
                        game_state.input_handler.main_pos,
                    );
                    game_state.camera.update_pos(
                        window_util::window_size(&render.res.window),
                        game_state.loop_store.elapsed.as_secs_f32(),
                    );
                    render
                        .renderer
                        .tile_tints
                        .insert(game_state.camera.pointing_at, colors::RED.with_alpha(0.2));

                    game_state.loop_store.frame_start = Some(now);

                    let result = util::render(closed, event_loop, game_state, render_state, render);

                    if render.screenshotting {
                        render.screenshotting = false;
                    }

                    return result;
                }
                WindowEvent::Resized(size) => {
                    render.res.resize(*size);
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    /* TODO reimpl
                    state.gui.as_mut().unwrap().yak.set_scale_factor(
                        (*scale_factor * state.options.graphics.ui_scale.to_f64()) as f32,
                    );
                    */
                }
                event => {
                    window_event = Some(event);
                }
            }
        }

        Event::DeviceEvent { event, .. } => {
            device_event = Some(event);
        }

        _ => {}
    };

    if window_event.is_some() || device_event.is_some() {
        let pointing_at_entity = game_state.loop_store.pointing_cache.blocking_lock().clone();

        game_state.input_handler.reset();

        game_state
            .input_handler
            .update(input::handler::convert_input(
                window_event,
                device_event,
                window_util::window_size(&render.res.window),
                1.0, //TODO sensitivity option
            ));

        game_state.camera.handle_input(&game_state.input_handler);

        game_state.input_hints.clear();

        game_state.input_hints.push(vec![ActionType::Cancel]);
        if game_state.input_handler.key_active(ActionType::Cancel) {
            // one by one
            if game_state.ui_state.selected_tile_id.take().is_none()
                && game_state.ui_state.linking_tile.take().is_none()
                && game_state.ui_state.paste_from.take().is_none()
            {
                if game_state
                    .ui_state
                    .switch_screen_if(ui::Screen::Paused, &|s| s.screen == ui::Screen::Ingame)
                {
                    game_state.game.send_message(GameMsg::SaveMap)?;
                } else {
                    game_state
                        .ui_state
                        .switch_screen_if(ui::Screen::Ingame, &|s| s.screen == ui::Screen::Paused);
                }
            }
        }

        game_state.input_hints.push(vec![ActionType::Player]);

        // TODO hint this
        if (game_state.input_handler.main_pressed
            || (game_state.input_handler.key_active(ActionType::SelectMode)
                && game_state.input_handler.main_held))
            && game_state.ui_state.already_placed_at != Some(game_state.camera.pointing_at)
            && let Some(id) = game_state.ui_state.selected_tile_id
        {
            util::place_tile(id, game_state.camera.pointing_at, game_state)?;
        }

        game_state.input_hints.push(vec![ActionType::Delete]);
        if game_state.input_handler.key_active(ActionType::Delete) {
            util::place_tile(
                TileId(game_state.resource_man.registry.none),
                game_state.camera.pointing_at,
                game_state,
            )?;
        }

        if !game_state.input_handler.key_active(ActionType::SelectMode) {
            // TODO hint this
            if game_state.input_handler.alternate_pressed {
                if let Some((id, link_from)) = game_state.ui_state.linking_tile.clone() {
                    util::link_tile(game_state, id, link_from, game_state.camera.pointing_at)?;
                } else if Some(game_state.camera.pointing_at) == game_state.ui_state.config_open_at
                {
                    game_state.ui_state.config_open_at = None;
                    game_state
                        .ui_state
                        .text_field
                        .get(TextField::Filter)
                        .clear();
                } else {
                    game_state.ui_state.config_open_at = Some(game_state.camera.pointing_at);
                    game_state
                        .ui_state
                        .text_field
                        .get(TextField::Filter)
                        .clear();
                }
            }
        }

        game_state.input_hints.push(vec![ActionType::SelectMode]);
        if game_state.input_handler.key_active(ActionType::SelectMode)
            && game_state.ui_state.screen == ui::Screen::Ingame
        {
            if game_state.ui_state.paste_from.is_none() {
                game_state
                    .ui_state
                    .grouped_tiles
                    .insert(game_state.camera.pointing_at);
            }
        } else {
            game_state.ui_state.grouped_tiles.clear();
        }

        if game_state
            .input_handler
            .key_active(ActionType::HotkeyActive)
        {
            game_state
                .input_hints
                .push(vec![ActionType::HotkeyActive, ActionType::Undo]);
            game_state
                .input_hints
                .push(vec![ActionType::HotkeyActive, ActionType::Redo]);

            if game_state.input_handler.key_active(ActionType::Undo) {
                game_state.game.send_message(GameMsg::Undo)?;
            }
            if game_state.input_handler.key_active(ActionType::Redo) {
                //TODO state.game.send_message(GameSystemMessage::Redo)?;
            }

            if game_state.ui_state.paste_from.is_none()
                && !game_state.ui_state.grouped_tiles.is_empty()
            {
                game_state
                    .input_hints
                    .push(vec![ActionType::HotkeyActive, ActionType::Cut]);
                game_state
                    .input_hints
                    .push(vec![ActionType::HotkeyActive, ActionType::Copy]);

                if game_state.input_handler.key_active(ActionType::Cut)
                    || game_state.input_handler.key_active(ActionType::Copy)
                {
                    game_state.ui_state.paste_from = Some(game_state.camera.pointing_at);
                    game_state
                        .audio_man
                        .play(game_state.resource_man.audio["click"].clone())?;

                    let coords =
                        Vec::from_iter(std::mem::take(&mut game_state.ui_state.grouped_tiles));

                    if game_state.input_handler.key_active(ActionType::Cut) {
                        let none = game_state.resource_man.registry.none;

                        game_state.ui_state.paste_content = game_state
                            .tokio
                            .block_on(game_state.game.call(
                                |reply| {
                                    GameMsg::PlaceTiles {
                                        tiles: coords
                                            .into_iter()
                                            .map(|coord| (coord, (TileId(none), DataMap::new())))
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
                        game_state.ui_state.paste_content = game_state
                            .tokio
                            .block_on(
                                game_state
                                    .game
                                    .call(|reply| GameMsg::GetTilesFlat(coords, reply), None),
                            )?
                            .unwrap();
                    }
                }
            }

            if let Some(start) = game_state.ui_state.paste_from {
                game_state
                    .input_hints
                    .push(vec![ActionType::HotkeyActive, ActionType::Paste]);

                if game_state.input_handler.key_active(ActionType::Paste) {
                    let direction = game_state.camera.pointing_at - start;

                    game_state.game.send_message(GameMsg::PlaceTiles {
                        tiles: game_state
                            .ui_state
                            .paste_content
                            .clone()
                            .into_iter()
                            .map(|(coord, tile)| (coord + direction, tile))
                            .collect::<FlatTiles>(),
                        replace: false,
                        record: true,
                        reply: None,
                    })?;

                    game_state
                        .audio_man
                        .play(game_state.resource_man.audio["click"].clone())?;
                    // TODO click2
                }
            }
        } else {
            game_state.input_hints.push(vec![ActionType::HotkeyActive]);
        }

        if game_state.input_handler.key_active(ActionType::Fullscreen) {
            game_state.options.graphics.fullscreen = !game_state.options.graphics.fullscreen;
            game_state.options.synced = false
        }

        game_state.input_hints.push(vec![ActionType::ToggleGui]);

        render.screenshotting = game_state.input_handler.key_active(ActionType::Screenshot);
    }

    Ok(())
}
