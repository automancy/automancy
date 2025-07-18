use std::{
    fs, mem,
    sync::atomic::Ordering,
    time::{Instant, SystemTime},
};

use automancy_defs::{
    coord::TileCoord,
    id::{Id, TileId},
    window,
};
use automancy_resources::data::Data;
use automancy_system::{
    game::{GameSystemMessage, PlaceTileResponse},
    input::{self, ActionType},
    map::{GameMap, LoadMapOption, MAP_PATH},
    tile_entity::{TileEntityMsg, TileEntityWithId},
    ui_state::{Screen, TextField},
};
use ractor::{ActorRef, rpc::CallResult};
use tokio::task::JoinHandle;
use wgpu::SurfaceError;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ActiveEventLoop,
};

use crate::{GameState, gui, renderer};

/// Refreshes the list of maps on the filesystem. Should be done every time the list of maps could have changed (on map creation/delete and on game load).
pub fn refresh_maps(state: &mut GameState) {
    fs::create_dir_all(MAP_PATH).unwrap();

    state.loop_store.map_infos_cache = fs::read_dir(MAP_PATH)
        .expect("Map folder doesn't exist- is the disk full?")
        .flatten()
        .map(|f| f.file_name().to_str().unwrap().to_string())
        .filter(|f| !f.starts_with('.'))
        .flat_map(|name| {
            GameMap::read_info(&state.resource_man, &LoadMapOption::FromSave(name.clone()))
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
    game: &ActorRef<GameSystemMessage>,
    game_handle: &mut Option<JoinHandle<()>>,
    event_loop: &ActiveEventLoop,
) -> anyhow::Result<bool> {
    game.send_message(GameSystemMessage::StopTicking)?;

    game.call(GameSystemMessage::SaveMap, None)
        .await
        .expect("Could not save the game on exit!");
    game.stop(Some("Game closed".to_string()));
    game_handle.take().unwrap().await?;

    event_loop.exit();

    log::info!("Shut down gracefully");

    Ok(true)
}

fn render(
    state: &mut GameState,
    event_loop: &ActiveEventLoop,
    screenshotting: bool,
) -> anyhow::Result<bool> {
    let mut result = Ok(false);

    {
        if !state
            .loop_store
            .config_open_updating
            .load(Ordering::Relaxed)
        {
            let config_open_at = state.ui_state.config_open_at;

            let cache = state.loop_store.config_open_cache.clone();
            let updating = state.loop_store.config_open_updating.clone();
            let game = state.game.clone();

            updating.store(true, Ordering::Relaxed);

            state.tokio.spawn(async move {
                let Some(config_open_at) = config_open_at else {
                    *cache.lock().await = None;
                    updating.store(false, Ordering::Relaxed);

                    return;
                };

                let Ok(CallResult::Success(entity)) = game
                    .call(
                        |reply| GameSystemMessage::GetTileEntity(config_open_at, reply),
                        None,
                    )
                    .await
                else {
                    return;
                };

                *cache.lock().await = entity;

                updating.store(false, Ordering::Relaxed);
            });
        }

        if !state.loop_store.pointing_updating.load(Ordering::Relaxed) {
            let cache = state.loop_store.pointing_cache.clone();
            let updating = state.loop_store.pointing_updating.clone();
            let game = state.game.clone();
            let pointing_at = state.camera.pointing_at;

            updating.store(true, Ordering::Relaxed);

            state.tokio.spawn(async move {
                let Ok(CallResult::Success(tile)) = game
                    .call(|reply| GameSystemMessage::GetTile(pointing_at, reply), None)
                    .await
                else {
                    return;
                };

                let Ok(CallResult::Success(entity)) = game
                    .call(
                        |reply| GameSystemMessage::GetTileEntity(pointing_at, reply),
                        None,
                    )
                    .await
                else {
                    return;
                };

                *cache.lock().await = tile.zip(entity);

                updating.store(false, Ordering::Relaxed);
            });
        }
    }

    {
        {
            state.gui.as_mut().unwrap().yak.start();

            gui::render_ui(state, &mut result, event_loop);

            state.gui.as_mut().unwrap().yak.finish();
        }

        if !matches!(result, Ok(true)) {
            match renderer::render(state, screenshotting) {
                Ok(_) => {}
                Err(SurfaceError::Lost) => {
                    let renderer = state.renderer.as_mut().unwrap();

                    renderer.gpu.resize(
                        &mut renderer.shared_resources,
                        &renderer.global_resources,
                        renderer.gpu.window.inner_size(),
                    );
                }
                Err(SurfaceError::OutOfMemory) => {
                    return state.tokio.block_on(shutdown_graceful(
                        &state.game,
                        &mut state.game_handle,
                        event_loop,
                    ));
                }
                Err(e) => log::error!("{e:?}"),
            }
        }
    }

    result
}

fn link_tile(state: &mut GameState, entity: Option<TileEntityWithId>, link_to: TileCoord, id: Id) {
    let Some((_, entity)) = entity else {
        return;
    };

    let Ok(CallResult::Success(old)) = state
        .tokio
        .block_on(entity.call(|reply| TileEntityMsg::GetDataValue(id, reply), None))
    else {
        return;
    };

    if old.is_some() {
        entity.send_message(TileEntityMsg::RemoveData(id)).unwrap();

        state
            .audio_man
            .play(state.resource_man.audio["click"].clone())
            .unwrap();
        // TODO click2
    } else {
        entity
            .send_message(TileEntityMsg::SetDataValue(id, Data::Coord(link_to)))
            .unwrap();

        state
            .audio_man
            .play(state.resource_man.audio["click"].clone())
            .unwrap();
    }
}

fn place_tile(id: TileId, coord: TileCoord, state: &mut GameState) -> anyhow::Result<()> {
    let response = state
        .tokio
        .block_on(state.game.call(
            |reply| GameSystemMessage::PlaceTile {
                coord,
                id,
                record: true,
                reply: Some(reply),
                data: None,
            },
            None,
        ))?
        .unwrap();

    match response {
        PlaceTileResponse::Placed => {
            state
                .audio_man
                .play(state.resource_man.audio["tile_placement"].clone())
                .unwrap();
            state.ui_state.config_open_at = Some(coord);
            state.ui_state.already_placed_at = Some(coord);
        }
        PlaceTileResponse::Removed => {
            state
                .audio_man
                .play(state.resource_man.audio["tile_removal"].clone())
                .unwrap();
        }
        _ => {}
    }

    Ok(())
}

/// Triggers every time the event loop is run once.
pub fn on_event(
    state: &mut GameState,
    event_loop: &ActiveEventLoop,
    event: Event<()>,
) -> anyhow::Result<bool> {
    let mut window_event = None;
    let mut device_event = None;

    match &event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            // game shutdown
            return state.tokio.block_on(shutdown_graceful(
                &state.game,
                &mut state.game_handle,
                event_loop,
            ));
        }
        Event::WindowEvent { event, window_id }
            if window_id == &state.renderer.as_ref().unwrap().gpu.window.id() =>
        {
            match event {
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();

                    state.loop_store.elapsed = now - state.loop_store.frame_start.take().unwrap();

                    state.camera.update_pointing_at(
                        state.input_handler.main_pos,
                        window::window_size_double(&state.renderer.as_ref().unwrap().gpu.window),
                    );
                    state.camera.update_pos(
                        window::window_size_double(&state.renderer.as_ref().unwrap().gpu.window),
                        state.loop_store.elapsed.as_secs_f32(),
                    );

                    state.loop_store.frame_start = Some(now);

                    let result = render(state, event_loop, state.screenshotting);

                    if state.screenshotting {
                        state.screenshotting = false;
                    }

                    return result;
                }
                WindowEvent::Resized(size) => {
                    let renderer = state.renderer.as_mut().unwrap();

                    renderer.gpu.resize(
                        &mut renderer.shared_resources,
                        &renderer.global_resources,
                        *size,
                    );

                    return Ok(false);
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    state.gui.as_mut().unwrap().yak.set_scale_factor(
                        (*scale_factor * state.options.graphics.ui_scale.to_f64()) as f32,
                    );
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
        let pointing_at_entity = state.loop_store.pointing_cache.blocking_lock().clone();

        state.input_handler.reset();

        state.input_handler.update(input::convert_input(
            window_event,
            device_event,
            window::window_size_double(&state.renderer.as_ref().unwrap().gpu.window),
            1.0, //TODO sensitivity option
        ));

        state.camera.handle_input(&state.input_handler);

        state.input_hints.clear();

        state.input_hints.push(vec![ActionType::Cancel]);
        if state.input_handler.key_active(ActionType::Cancel) {
            // one by one
            if state.ui_state.selected_tile_id.take().is_none()
                && state.ui_state.linking_tile.take().is_none()
                && state.ui_state.paste_from.take().is_none()
            {
                if state
                    .ui_state
                    .switch_screen_when(&|s| s.screen == Screen::Ingame, Screen::Paused)
                {
                    state
                        .tokio
                        .block_on(state.game.call(GameSystemMessage::SaveMap, None))?
                        .unwrap();
                } else {
                    state
                        .ui_state
                        .switch_screen_when(&|s| s.screen == Screen::Paused, Screen::Ingame);
                }
            }
        }

        state.input_hints.push(vec![ActionType::Player]);

        // TODO hint this
        if (state.input_handler.main_pressed
            || (state.input_handler.key_active(ActionType::SelectMode)
                && state.input_handler.main_held))
            && state.ui_state.already_placed_at != Some(state.camera.pointing_at)
        {
            if let Some(id) = state.ui_state.selected_tile_id {
                place_tile(id, state.camera.pointing_at, state)?;
            }
        }

        state.input_hints.push(vec![ActionType::Delete]);
        if state.input_handler.key_active(ActionType::Delete) {
            place_tile(
                TileId(state.resource_man.registry.none),
                state.camera.pointing_at,
                state,
            )?;
        }

        if !state.input_handler.key_active(ActionType::SelectMode) {
            // TODO hint this
            if state.input_handler.alternate_pressed {
                if let Some((link_to, id)) = state.ui_state.linking_tile {
                    link_tile(state, pointing_at_entity, link_to, id);
                } else if Some(state.camera.pointing_at) == state.ui_state.config_open_at {
                    state.ui_state.config_open_at = None;
                    state.ui_state.text_field.get(TextField::Filter).clear();
                } else {
                    state.ui_state.config_open_at = Some(state.camera.pointing_at);
                    state.ui_state.text_field.get(TextField::Filter).clear();
                }
            }
        }

        state.input_hints.push(vec![ActionType::SelectMode]);
        if state.input_handler.key_active(ActionType::SelectMode)
            && state.ui_state.screen == Screen::Ingame
        {
            if state.ui_state.paste_from.is_none() {
                state
                    .ui_state
                    .grouped_tiles
                    .insert(state.camera.pointing_at);
            }
        } else {
            state.ui_state.grouped_tiles.clear();
        }

        if state.input_handler.key_active(ActionType::HotkeyActive) {
            state
                .input_hints
                .push(vec![ActionType::HotkeyActive, ActionType::Undo]);
            state
                .input_hints
                .push(vec![ActionType::HotkeyActive, ActionType::Redo]);

            if state.input_handler.key_active(ActionType::Undo) {
                state.game.send_message(GameSystemMessage::Undo)?;
            }
            if state.input_handler.key_active(ActionType::Redo) {
                //TODO state.game.send_message(GameSystemMessage::Redo)?;
            }

            if state.ui_state.paste_from.is_none() && !state.ui_state.grouped_tiles.is_empty() {
                state
                    .input_hints
                    .push(vec![ActionType::HotkeyActive, ActionType::Cut]);
                state
                    .input_hints
                    .push(vec![ActionType::HotkeyActive, ActionType::Copy]);

                if state.input_handler.key_active(ActionType::Cut)
                    || state.input_handler.key_active(ActionType::Copy)
                {
                    state.ui_state.paste_from = Some(state.camera.pointing_at);
                    state
                        .audio_man
                        .play(state.resource_man.audio["click"].clone())?;

                    let coords = Vec::from_iter(mem::take(&mut state.ui_state.grouped_tiles));

                    if state.input_handler.key_active(ActionType::Cut) {
                        let none = state.resource_man.registry.none;

                        state.ui_state.paste_content = state
                            .tokio
                            .block_on(state.game.call(
                                |reply| {
                                    GameSystemMessage::PlaceTiles {
                                        tiles: coords
                                            .into_iter()
                                            .map(|coord| (coord, TileId(none), None))
                                            .collect::<Vec<_>>(),
                                        reply: Some(reply),
                                        place_over: true,
                                        record: true,
                                    }
                                },
                                None,
                            ))?
                            .unwrap();
                    } else {
                        state.ui_state.paste_content = state
                            .tokio
                            .block_on(
                                state
                                    .game
                                    .call(|reply| GameSystemMessage::GetTiles(coords, reply), None),
                            )?
                            .unwrap();
                    }
                }
            }

            if let Some(start) = state.ui_state.paste_from {
                state
                    .input_hints
                    .push(vec![ActionType::HotkeyActive, ActionType::Paste]);

                if state.input_handler.key_active(ActionType::Paste) {
                    let direction = state.camera.pointing_at - start;

                    let tiles = state
                        .ui_state
                        .paste_content
                        .clone()
                        .into_iter()
                        .map(|(coord, id, data)| (coord + direction, id, data))
                        .collect::<Vec<_>>();

                    state.game.send_message(GameSystemMessage::PlaceTiles {
                        tiles,
                        reply: None,
                        place_over: false,
                        record: true,
                    })?;

                    state
                        .audio_man
                        .play(state.resource_man.audio["click"].clone())?;
                    // TODO click2
                }
            }
        } else {
            state.input_hints.push(vec![ActionType::HotkeyActive]);
        }

        if state.input_handler.key_active(ActionType::Fullscreen) {
            state.options.graphics.fullscreen = !state.options.graphics.fullscreen;
            state.options.synced = false
        }

        state.screenshotting = state.input_handler.key_active(ActionType::Screenshot);

        state.input_hints.push(vec![ActionType::ToggleGui]);
    }

    Ok(false)
}
