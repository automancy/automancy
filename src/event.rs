use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use std::{fs, mem};

use hashbrown::HashMap;
use ractor::rpc::CallResult;
use ractor::ActorRef;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use wgpu::SurfaceError;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ActiveEventLoop,
};

use automancy_defs::coord::TileCoord;
use automancy_defs::id::Id;
use automancy_defs::{log, window};
use automancy_resources::{data::Data, types::item::ItemDef};

use crate::gui::{Screen, TextField};
use crate::input::ActionType;
use crate::map::{Map, MapInfo, MapInfoRaw, MAP_PATH};
use crate::tile_entity::TileEntityMsg;
use crate::{
    game::{GameSystemMessage, PlaceTileResponse},
    renderer::Renderer,
};
use crate::{gui, input, GameState};

pub type TileEntityWithId = (Id, ActorRef<TileEntityMsg>);

/// Refreshes the list of maps on the filesystem. Should be done every time the list of maps could have changed (on map creation/delete and on game load).
pub fn refresh_maps(state: &mut GameState) {
    drop(fs::create_dir_all(MAP_PATH));

    state.loop_store.map_infos_cache = fs::read_dir(MAP_PATH)
        .expect("Map folder doesn't exist- is the disk full?")
        .flatten()
        .map(|f| f.file_name().to_str().unwrap().to_string())
        .filter(|f| !f.starts_with('.'))
        .flat_map(|map| {
            Map::read_info(&state.resource_man, &map)
                .ok()
                .zip(Some(map))
        })
        .collect::<Vec<_>>();

    state
        .loop_store
        .map_infos_cache
        .sort_by(|a, b| a.1.cmp(&b.1));
    state.loop_store.map_infos_cache.sort_by(|a, b| {
        a.0 .1
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .cmp(&b.0 .1.unwrap_or(SystemTime::UNIX_EPOCH))
    });
    state.loop_store.map_infos_cache.reverse();
}

/// Stores information that lives for the entire lifetime of the session, and is not dropped at the end of one event cycle or handled elsewhere.
#[derive(Debug, Default)]
pub struct EventLoopStorage {
    /// tag searching cache
    pub tag_cache: HashMap<Id, Arc<Vec<ItemDef>>>,
    /// the last frame's starting time
    pub frame_start: Option<Instant>,
    /// the elapsed time between each frame
    pub elapsed: Duration,

    pub map_infos_cache: Vec<((MapInfoRaw, Option<SystemTime>), String)>,
    pub map_info: Option<(Arc<Mutex<MapInfo>>, String)>,

    pub config_open_cache: Arc<Mutex<Option<ActorRef<TileEntityMsg>>>>,
    pub config_open_updating: Arc<AtomicBool>,
    pub pointing_cache: Arc<Mutex<Option<TileEntityWithId>>>,
    pub pointing_updating: Arc<AtomicBool>,
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

    state.loop_store.frame_start = Some(Instant::now());

    {
        if !state
            .loop_store
            .config_open_updating
            .load(Ordering::Relaxed)
        {
            let config_open_at = state.gui_state.config_open_at;

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
            match Renderer::render(state, screenshotting) {
                Ok(_) => {}
                Err(SurfaceError::Lost) => {
                    let renderer = state.renderer.as_mut().unwrap();

                    renderer.gpu.resize(
                        &mut renderer.shared_resources,
                        &renderer.global_resources,
                        renderer.gpu.window.inner_size(),
                    )
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

fn place_tile(id: Id, coord: TileCoord, state: &mut GameState) -> anyhow::Result<()> {
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
            state.gui_state.config_open_at = Some(coord);
            state.gui_state.already_placed_at = Some(coord);
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
                    state.loop_store.elapsed =
                        Instant::now().duration_since(state.loop_store.frame_start.take().unwrap());

                    state.camera.update_pointing_at(
                        state.input_handler.main_pos,
                        window::window_size_double(&state.renderer.as_ref().unwrap().gpu.window),
                    );
                    state.camera.update_pos(
                        window::window_size_double(&state.renderer.as_ref().unwrap().gpu.window),
                        state.loop_store.elapsed.as_secs_f64(),
                    );

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
            if state.gui_state.selected_tile_id.take().is_none()
                && state.gui_state.linking_tile.take().is_none()
                && state.gui_state.paste_from.take().is_none()
            {
                if state
                    .gui_state
                    .switch_screen_when(&|s| s.screen == Screen::Ingame, Screen::Paused)
                {
                    state
                        .tokio
                        .block_on(state.game.call(GameSystemMessage::SaveMap, None))?
                        .unwrap();
                } else {
                    state
                        .gui_state
                        .switch_screen_when(&|s| s.screen == Screen::Paused, Screen::Ingame);
                }
            }
        }

        // TODO hint this
        if (state.input_handler.main_pressed
            || (state.input_handler.key_active(ActionType::SelectMode)
                && state.input_handler.main_held))
            && state.gui_state.already_placed_at != Some(state.camera.pointing_at)
        {
            if let Some(id) = state.gui_state.selected_tile_id {
                place_tile(id, state.camera.pointing_at, state)?;
            }
        }

        state.input_hints.push(vec![ActionType::Delete]);
        if state.input_handler.key_active(ActionType::Delete) {
            place_tile(
                state.resource_man.registry.none,
                state.camera.pointing_at,
                state,
            )?;
        }

        if !state.input_handler.key_active(ActionType::SelectMode) {
            // TODO hint this
            if state.input_handler.alternate_pressed {
                if let Some((link_to, id)) = state.gui_state.linking_tile {
                    link_tile(state, pointing_at_entity, link_to, id);
                } else if Some(state.camera.pointing_at) == state.gui_state.config_open_at {
                    state.gui_state.config_open_at = None;
                    state.gui_state.text_field.get(TextField::Filter).clear();
                } else {
                    state.gui_state.config_open_at = Some(state.camera.pointing_at);
                    state.gui_state.text_field.get(TextField::Filter).clear();
                }
            }
        }

        state.input_hints.push(vec![ActionType::SelectMode]);
        if state.input_handler.key_active(ActionType::SelectMode)
            && state.gui_state.screen == Screen::Ingame
        {
            if state.gui_state.paste_from.is_none() {
                state
                    .gui_state
                    .grouped_tiles
                    .insert(state.camera.pointing_at);
            }
        } else {
            state.gui_state.grouped_tiles.clear();
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

            if state.gui_state.paste_from.is_none() && !state.gui_state.grouped_tiles.is_empty() {
                state
                    .input_hints
                    .push(vec![ActionType::HotkeyActive, ActionType::Cut]);
                state
                    .input_hints
                    .push(vec![ActionType::HotkeyActive, ActionType::Copy]);

                if state.input_handler.key_active(ActionType::Cut)
                    || state.input_handler.key_active(ActionType::Copy)
                {
                    state.gui_state.paste_from = Some(state.camera.pointing_at);
                    state
                        .audio_man
                        .play(state.resource_man.audio["click"].clone())?;

                    let coords = Vec::from_iter(mem::take(&mut state.gui_state.grouped_tiles));

                    if state.input_handler.key_active(ActionType::Cut) {
                        let none = state.resource_man.registry.none;

                        state.gui_state.paste_content = state
                            .tokio
                            .block_on(state.game.call(
                                |reply| {
                                    GameSystemMessage::PlaceTiles {
                                        tiles: coords
                                            .into_iter()
                                            .map(|coord| (coord, none, None))
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
                        state.gui_state.paste_content = state
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

            if let Some(start) = state.gui_state.paste_from {
                state
                    .input_hints
                    .push(vec![ActionType::HotkeyActive, ActionType::Paste]);

                if state.input_handler.key_active(ActionType::Paste) {
                    let direction = state.camera.pointing_at - start;

                    let tiles = state
                        .gui_state
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
