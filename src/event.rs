use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use std::{fs, mem};

use hashbrown::{HashMap, HashSet};
use ractor::rpc::CallResult;
use ractor::ActorRef;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use wgpu::SurfaceError;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;

use automancy_defs::coord::TileCoord;
use automancy_defs::hexx::Hex;
use automancy_defs::id::Id;
use automancy_defs::{log, math, window};
use automancy_resources::data::item::Item;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::kira::manager::AudioManager;
use automancy_resources::ResourceManager;

use crate::game::{GameSystemMessage, PlaceTileResponse};
use crate::gui::{Screen, TextField};
use crate::input::KeyActions;
use crate::map::{Map, MapInfo, MapInfoRaw, MAP_PATH};
use crate::tile_entity::TileEntityMsg;
use crate::{gui, input, GameState};

/// Refreshes the list of maps on the filesystem. Should be done every time the list of maps could have changed (on map creation/delete and on game load).
pub fn refresh_maps(state: &mut GameState) {
    drop(fs::create_dir_all(MAP_PATH));

    state.loop_store.map_infos_cache = fs::read_dir(MAP_PATH)
        .expect("Map folder doesn't exist- is the disk full?")
        .flatten()
        .map(|f| f.file_name().to_str().unwrap().to_string())
        .filter(|f| !f.starts_with('.'))
        .flat_map(|map| Map::read_info(&state.resource_man, &map).zip(Some(map)))
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
    pub tag_cache: HashMap<Id, Arc<Vec<Item>>>,
    /// the last frame's starting time
    pub frame_start: Option<Instant>,
    /// the elapsed time between each frame
    pub elapsed: Duration,

    pub map_infos_cache: Vec<((MapInfoRaw, Option<SystemTime>), String)>,
    pub map_info: Option<(Arc<Mutex<MapInfo>>, String)>,

    pub config_open_cache: Arc<Mutex<Option<(Id, ActorRef<TileEntityMsg>)>>>,
    pub config_open_updating: Arc<AtomicBool>,
    pub pointing_cache: Arc<Mutex<Option<(Id, ActorRef<TileEntityMsg>)>>>,
    pub pointing_updating: Arc<AtomicBool>,
}

pub async fn shutdown_graceful(
    game: &ActorRef<GameSystemMessage>,
    game_handle: &mut Option<JoinHandle<()>>,
    target: &EventLoopWindowTarget<()>,
) -> anyhow::Result<bool> {
    game.send_message(GameSystemMessage::StopTicking)?;

    game.call(GameSystemMessage::SaveMap, None).await.unwrap();
    game.stop(Some("Game closed".to_string()));
    game_handle.take().unwrap().await?;

    target.exit();

    log::info!("Shut down gracefully");

    Ok(true)
}

fn render(state: &mut GameState, target: &EventLoopWindowTarget<()>) -> anyhow::Result<bool> {
    let mut result = Ok(false);

    state.camera.update_pointing_at(
        state.input_handler.main_pos,
        window::window_size_double(&state.renderer.gpu.window),
    );
    state.camera.update_pos(
        window::window_size_double(&state.renderer.gpu.window),
        state.loop_store.elapsed.as_secs_f64(),
    );

    state.loop_store.frame_start = Some(Instant::now());

    {
        if let Some(config_open_at) = state.gui_state.config_open_at {
            if !state
                .loop_store
                .config_open_updating
                .load(Ordering::Relaxed)
            {
                let cache = state.loop_store.config_open_cache.clone();
                let updating = state.loop_store.config_open_updating.clone();
                let game = state.game.clone();

                updating.store(true, Ordering::Relaxed);

                state.tokio.spawn(async move {
                    let tile = game
                        .call(
                            |reply| GameSystemMessage::GetTile(config_open_at, reply),
                            None,
                        )
                        .await
                        .unwrap()
                        .unwrap();

                    let entity = game
                        .call(
                            |reply| GameSystemMessage::GetTileEntity(config_open_at, reply),
                            None,
                        )
                        .await
                        .unwrap()
                        .unwrap();

                    *cache.lock().await = tile.zip(entity);

                    updating.store(false, Ordering::Relaxed);
                });
            }
        }

        if !state.loop_store.pointing_updating.load(Ordering::Relaxed) {
            let cache = state.loop_store.pointing_cache.clone();
            let updating = state.loop_store.pointing_updating.clone();
            let game = state.game.clone();
            let pointing_at = state.camera.pointing_at;

            updating.store(true, Ordering::Relaxed);

            state.tokio.spawn(async move {
                let tile = game
                    .call(|reply| GameSystemMessage::GetTile(pointing_at, reply), None)
                    .await
                    .unwrap()
                    .unwrap();

                let entity = game
                    .call(
                        |reply| GameSystemMessage::GetTileEntity(pointing_at, reply),
                        None,
                    )
                    .await
                    .unwrap()
                    .unwrap();

                *cache.lock().await = tile.zip(entity);

                updating.store(false, Ordering::Relaxed);
            });
        }
    }

    {
        {
            state.gui.yak.start();

            gui::render_ui(state, &mut result, target);

            state.gui.yak.finish();
        }

        if !matches!(result, Ok(true)) {
            match state.renderer.render(
                state.start_instant,
                state.resource_man.clone(),
                &state.tokio,
                &state.input_handler,
                &state.camera,
                &mut state.gui,
                &state.game,
            ) {
                Ok(_) => {}
                Err(SurfaceError::Lost) => state.renderer.gpu.resize(
                    &mut state.renderer.shared_resources,
                    &mut state.renderer.render_resources,
                    state.renderer.gpu.window.inner_size(),
                ),
                Err(SurfaceError::OutOfMemory) => {
                    return state.tokio.block_on(shutdown_graceful(
                        &state.game,
                        &mut state.game_handle,
                        target,
                    ));
                }
                Err(e) => log::error!("{e:?}"),
            }
        }
    }

    result
}

async fn on_link_tile(
    resource_man: Arc<ResourceManager>,
    audio_man: &mut AudioManager,
    pointing_cache: Arc<Mutex<Option<(Id, ActorRef<TileEntityMsg>)>>>,
    linking_tile: TileCoord,
) {
    let Some((tile, entity)) = pointing_cache.lock().await.clone() else {
        return;
    };

    let Some(tile_def) = resource_man.registry.tiles.get(&tile) else {
        return;
    };

    if tile_def
        .data
        .get(&resource_man.registry.data_ids.linked)
        .cloned()
        .and_then(Data::into_bool)
        .unwrap_or(false)
    {
        let Ok(CallResult::Success(old)) = entity
            .call(
                |reply| TileEntityMsg::GetDataValue(resource_man.registry.data_ids.link, reply),
                None,
            )
            .await
        else {
            return;
        };

        if old.is_some() {
            entity
                .send_message(TileEntityMsg::RemoveData(
                    resource_man.registry.data_ids.link,
                ))
                .unwrap();

            audio_man.play(resource_man.audio["click"].clone()).unwrap();
            // TODO click2
        } else {
            entity
                .send_message(TileEntityMsg::SetDataValue(
                    resource_man.registry.data_ids.link,
                    Data::Coord(linking_tile),
                ))
                .unwrap();

            audio_man.play(resource_man.audio["click"].clone()).unwrap();
        }
    }
}

/// Triggers every time the event loop is run once.
pub fn on_event(
    state: &mut GameState,
    target: &EventLoopWindowTarget<()>,
    event: Event<()>,
) -> anyhow::Result<bool> {
    let mut window_event = None;
    let mut device_event = None;

    if !state.gui.window.handle_event(&mut state.gui.yak, &event) {
        match &event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                // game shutdown
                return state.tokio.block_on(shutdown_graceful(
                    &state.game,
                    &mut state.game_handle,
                    target,
                ));
            }

            Event::WindowEvent { event, window_id }
                if window_id == &state.renderer.gpu.window.id() =>
            {
                window_event = Some(event);

                match event {
                    WindowEvent::RedrawRequested => {
                        state.renderer.gpu.window.pre_present_notify();

                        return render(state, target);
                    }
                    WindowEvent::Resized(size) => {
                        state.renderer.gpu.resize(
                            &mut state.renderer.shared_resources,
                            &mut state.renderer.render_resources,
                            *size,
                        );

                        return Ok(false);
                    }
                    _ => {}
                }
            }

            Event::DeviceEvent { event, .. } => {
                device_event = Some(event);
            }

            _ => {}
        };
    };

    if window_event.is_some() || device_event.is_some() {
        state.input_handler.reset();
        state.input_handler.update(input::convert_input(
            window_event,
            device_event,
            window::window_size_double(&state.renderer.gpu.window),
            1.0, //TODO sensitivity option
        ));
        state.camera.handle_input(&state.input_handler);

        if state.input_handler.key_active(KeyActions::Escape) {
            // one by one
            if state.gui_state.selected_tile_id.take().is_none()
                && state.gui_state.linking_tile.take().is_none()
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

        if state.input_handler.main_pressed
            || (state.input_handler.shift_held && state.input_handler.main_held)
        {
            if let Some(id) = state.gui_state.selected_tile_id {
                let mut data = DataMap::default();

                if let Some(mut coord) = state.gui_state.placement_direction.take() {
                    if let Some(old) = state.gui_state.prev_placement_direction.replace(coord) {
                        if old == -coord {
                            coord = old;
                            state.gui_state.prev_placement_direction.replace(old);
                        }
                    }

                    data.insert(
                        state.resource_man.registry.data_ids.direction,
                        Data::Coord(coord),
                    );
                } else {
                    state.gui_state.prev_placement_direction = None;
                }

                if state.gui_state.already_placed_at != Some(state.camera.pointing_at) {
                    let response = state
                        .tokio
                        .block_on(state.game.call(
                            |reply| GameSystemMessage::PlaceTile {
                                coord: state.camera.pointing_at,
                                id,
                                record: true,
                                reply: Some(reply),
                                data: Some(data),
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
                        }
                        PlaceTileResponse::Removed => {
                            state
                                .audio_man
                                .play(state.resource_man.audio["tile_removal"].clone())
                                .unwrap();
                        }
                        _ => {}
                    }

                    state.gui_state.already_placed_at = Some(state.camera.pointing_at)
                }
            }
        }

        if !state.input_handler.control_held && state.input_handler.alternate_pressed {
            if let Some(linking_tile) = state.gui_state.linking_tile {
                state.tokio.block_on(on_link_tile(
                    state.resource_man.clone(),
                    &mut state.audio_man,
                    state.loop_store.pointing_cache.clone(),
                    linking_tile,
                ));
            } else if Some(state.camera.pointing_at) == state.gui_state.config_open_at {
                state.gui_state.config_open_at = None;
                state.gui_state.text_field.get(TextField::Filter).clear();
            } else {
                state.gui_state.config_open_at = Some(state.camera.pointing_at);
            }
        }

        if state.input_handler.control_held && state.gui_state.screen == Screen::Ingame {
            if let Some(start) = state.gui_state.initial_cursor_position {
                if state.input_handler.alternate_pressed {
                    let direction = state.camera.pointing_at - start;

                    state.game.send_message(GameSystemMessage::MoveTiles(
                        state
                            .gui_state
                            .grouped_tiles
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>(),
                        direction,
                        true,
                    ))?;

                    let cap = state.gui_state.grouped_tiles.capacity();
                    for selected in mem::replace(
                        &mut state.gui_state.grouped_tiles,
                        HashSet::with_capacity(cap),
                    ) {
                        let dest = selected + direction;

                        state.gui_state.grouped_tiles.insert(dest);
                    }

                    state.gui_state.initial_cursor_position = None;
                    state
                        .audio_man
                        .play(state.resource_man.audio["click"].clone())?; // TODO click2
                }
            } else if state.input_handler.alternate_pressed {
                state.gui_state.initial_cursor_position = Some(state.camera.pointing_at);
                state
                    .audio_man
                    .play(state.resource_man.audio["click"].clone())?;
            }

            if state.gui_state.initial_cursor_position.is_none() {
                state
                    .gui_state
                    .grouped_tiles
                    .insert(state.camera.pointing_at);
            }
        } else {
            state.gui_state.grouped_tiles.clear();
            state.gui_state.initial_cursor_position = None;
        }

        if state.input_handler.control_held && state.input_handler.key_active(KeyActions::Undo) {
            state.game.send_message(GameSystemMessage::Undo)?;
        }

        if state.input_handler.key_active(KeyActions::Fullscreen) {
            state.options.graphics.fullscreen = !state.options.graphics.fullscreen;
            state.options.synced = false
        }

        if let Some(selected_tile_id) = state.gui_state.selected_tile_id {
            if state.input_handler.shift_held
                && !state.resource_man.registry.tiles[&selected_tile_id]
                    .data
                    .get(&state.resource_man.registry.data_ids.indirectional)
                    .cloned()
                    .and_then(Data::into_bool)
                    .unwrap_or(false)
            {
                let hex = math::main_pos_to_fract_hex(
                    window::window_size_double(&state.renderer.gpu.window),
                    state.input_handler.main_pos,
                    state.camera.get_pos(),
                );
                let rounded = Hex::round(hex.to_array()).as_vec2();
                let fract = (hex - rounded) * 2.0;

                state.gui_state.placement_direction = Some(Hex::round(fract.to_array()).into())
            } else {
                state.gui_state.placement_direction = None;
                state.gui_state.prev_placement_direction = None;
            }
        }
    }

    Ok(false)
}
