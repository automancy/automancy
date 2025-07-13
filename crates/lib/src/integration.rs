use std::{sync::Arc, time::Instant};

use automancy_data::{
    game::generic::DataMap,
    id::TileId,
    math::{UVec2, Vec2},
};
use automancy_game::{
    actor::{FlatTiles, message::GameMsg},
    input::{ActionType, GameInputEvent},
    resources,
    resources::{RESOURCES_PATH, ResourceManager},
    state::{AutomancyGameState, ui, ui::TextField},
};
use automancy_rendering::renderer::{AutomancyRenderState, AutomancyRendering};
use kira::track::TrackHandle;
use ractor::rpc::CallResult;
use winit::{event::Event, window::Window};

pub trait WindowExt {
    fn size_vec2(&self) -> Vec2;
    fn size_uvec2(&self) -> UVec2;
}

impl WindowExt for Window {
    fn size_uvec2(&self) -> UVec2 {
        let size = self.inner_size();

        UVec2::new(size.width, size.height)
    }

    fn size_vec2(&self) -> Vec2 {
        let size = self.inner_size();

        Vec2::new(size.width as f32, size.height as f32)
    }
}

#[inline]
pub fn render(
    window: &Window,
    surface: &wgpu::SurfaceTexture,
    render: &mut AutomancyRendering,
    game_state: &mut AutomancyGameState,
    render_state: &mut AutomancyRenderState,
    encoder: &mut wgpu::CommandEncoder,
) {
    game_state
        .camera
        .update_pointing_at(window.size_vec2(), game_state.input_handler.main_pos);

    {
        if let Ok(mut cache) = game_state.game_data.config_open_cache.clone().try_lock_owned() {
            let game_handle = game_state.game_handle.clone();
            let config_open_at = game_state.ui_state.config_open_at;

            game_state.tokio.spawn(async move {
                let Some(config_open_at) = config_open_at else {
                    *cache = None;

                    return;
                };

                let Ok(CallResult::Success(tile)) = game_handle.call(|reply| GameMsg::GetTile(config_open_at, reply), None).await else {
                    return;
                };

                *cache = tile.map(|v| v.handle);
            });
        }

        if let Ok(mut cache) = game_state.game_data.pointing_cache.clone().try_lock_owned() {
            let game_handle = game_state.game_handle.clone();
            let pointing_at = game_state.camera.pointing_at;

            game_state.tokio.spawn(async move {
                let Ok(CallResult::Success(tile)) = game_handle.call(|reply| GameMsg::GetTile(pointing_at, reply), None).await else {
                    return;
                };

                *cache = tile;
            });
        }
    }

    /*
    {
        state.gui.as_mut().unwrap().yak.start();

        gui::render_ui(state, &mut result, event_loop);

        state.gui.as_mut().unwrap().yak.finish();
    }
     */

    {
        let elapsed = render.frame_start.elapsed();
        game_state.camera.update_pos(window.size_vec2(), elapsed.as_secs_f32());
    }

    render.frame_start = Instant::now();
    render.renderer.render(surface, game_state, render_state, &mut render.res, encoder);
}

pub fn handle_winit_event(
    window: &Window,
    render: &mut AutomancyRendering,
    game_state: &mut AutomancyGameState,
    event: Event<()>,
) -> anyhow::Result<()> {
    let mut window_event = None;
    let mut device_event = None;

    match &event {
        Event::WindowEvent { event, window_id } if window_id == &window.id() => {
            window_event = Some(event);
        }

        Event::DeviceEvent { event, .. } => {
            device_event = Some(event);
        }

        _ => {}
    };

    if window_event.is_some() || device_event.is_some() {
        let pointing_at_entity = game_state.game_data.pointing_cache.blocking_lock().clone();

        game_state.input_handler.reset();

        game_state.input_handler.update(GameInputEvent::from_winit_event(
            window_event,
            device_event,
            window.size_vec2(),
            1.0, //TODO sensitivity option
        ));

        game_state.camera.handle_input(&game_state.input_handler);

        game_state.game_data.input_hints.clear();

        game_state.game_data.input_hints.push(vec![ActionType::Cancel]);
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
                    game_state.game_handle.send_message(GameMsg::SaveMap)?;
                } else {
                    game_state
                        .ui_state
                        .switch_screen_if(ui::Screen::Ingame, &|s| s.screen == ui::Screen::Paused);
                }
            }
        }

        game_state.game_data.input_hints.push(vec![ActionType::Player]);

        // TODO hint this
        if (game_state.input_handler.main_pressed
            || (game_state.input_handler.key_active(ActionType::SelectMode) && game_state.input_handler.main_held))
            && game_state.ui_state.last_placed_at != Some(game_state.camera.pointing_at)
            && let Some(id) = game_state.ui_state.selected_tile_id
        {
            game_state.place_tile(id, game_state.camera.pointing_at)?;
        }

        game_state.game_data.input_hints.push(vec![ActionType::Delete]);
        if game_state.input_handler.key_active(ActionType::Delete) {
            game_state.place_tile(TileId(game_state.resource_man.registry.none), game_state.camera.pointing_at)?;
        }

        if !game_state.input_handler.key_active(ActionType::SelectMode) {
            // TODO hint this
            if game_state.input_handler.alternate_pressed {
                if let Some((id, link_from)) = game_state.ui_state.linking_tile.clone() {
                    game_state.link_tile(id, link_from, game_state.camera.pointing_at)?;
                } else if Some(game_state.camera.pointing_at) == game_state.ui_state.config_open_at {
                    game_state.ui_state.config_open_at = None;
                    game_state.ui_state.text_field.get(TextField::Filter).clear();
                } else {
                    game_state.ui_state.config_open_at = Some(game_state.camera.pointing_at);
                    game_state.ui_state.text_field.get(TextField::Filter).clear();
                }
            }
        }

        game_state.game_data.input_hints.push(vec![ActionType::SelectMode]);
        if game_state.input_handler.key_active(ActionType::SelectMode) && game_state.ui_state.screen == ui::Screen::Ingame {
            if game_state.ui_state.paste_from.is_none() {
                game_state.ui_state.grouped_tiles.insert(game_state.camera.pointing_at);
            }
        } else {
            game_state.ui_state.grouped_tiles.clear();
        }

        if game_state.input_handler.key_active(ActionType::HotkeyActive) {
            game_state.game_data.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Undo]);
            game_state.game_data.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Redo]);

            if game_state.input_handler.key_active(ActionType::Undo) {
                game_state.game_handle.send_message(GameMsg::Undo)?;
            }
            if game_state.input_handler.key_active(ActionType::Redo) {
                //TODO state.game.send_message(GameSystemMessage::Redo)?;
            }

            if game_state.ui_state.paste_from.is_none() && !game_state.ui_state.grouped_tiles.is_empty() {
                game_state.game_data.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Cut]);
                game_state.game_data.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Copy]);

                if game_state.input_handler.key_active(ActionType::Cut) || game_state.input_handler.key_active(ActionType::Copy) {
                    game_state.ui_state.paste_from = Some(game_state.camera.pointing_at);
                    game_state.audio_man.play(game_state.resource_man.audio["click"].clone())?;

                    let coords = Vec::from_iter(std::mem::take(&mut game_state.ui_state.grouped_tiles));

                    if game_state.input_handler.key_active(ActionType::Cut) {
                        let none = game_state.resource_man.registry.none;

                        game_state.ui_state.paste_content = game_state
                            .tokio
                            .block_on(game_state.game_handle.call(
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
                            .block_on(game_state.game_handle.call(|reply| GameMsg::GetTilesFlat(coords, reply), None))?
                            .unwrap();
                    }
                }
            }

            if let Some(start) = game_state.ui_state.paste_from {
                game_state.game_data.input_hints.push(vec![ActionType::HotkeyActive, ActionType::Paste]);

                if game_state.input_handler.key_active(ActionType::Paste) {
                    let direction = game_state.camera.pointing_at - start;

                    game_state.game_handle.send_message(GameMsg::PlaceTiles {
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

                    game_state.audio_man.play(game_state.resource_man.audio["click"].clone())?;
                    // TODO click2
                }
            }
        } else {
            game_state.game_data.input_hints.push(vec![ActionType::HotkeyActive]);
        }

        if game_state.input_handler.key_active(ActionType::Fullscreen) {
            game_state.options.graphics.fullscreen = !game_state.options.graphics.fullscreen;
            game_state.options.synced = false
        }

        game_state.game_data.input_hints.push(vec![ActionType::ToggleGui]);

        render.screenshotting = game_state.input_handler.key_active(ActionType::Screenshot);
    }

    Ok(())
}

pub fn load_resources(lang: &str, track: TrackHandle) -> Arc<ResourceManager> {
    let mut resource_man = ResourceManager::new(track);

    std::fs::read_dir(RESOURCES_PATH)
        .expect("the resources folder needs to exist and be readable")
        .flatten()
        .map(|v| v.path())
        .filter(|v| v.is_dir())
        .for_each(|dir| {
            let namespace = dir.file_name().unwrap().to_str().unwrap().trim();
            log::info!("Loading namespace {namespace}...");

            resource_man.load_models(&dir, namespace).unwrap();
            resource_man.load_audio(&dir).unwrap();
            resource_man.load_tiles(&dir, namespace).unwrap();
            resource_man.load_items(&dir, namespace).unwrap();
            resource_man.load_tags(&dir, namespace).unwrap();
            resource_man.load_categories(&dir, namespace).unwrap();
            resource_man.load_recipes(&dir, namespace).unwrap();
            resource_man.load_translates(&dir, namespace, lang).unwrap();
            resource_man.load_shaders(&dir).unwrap();
            resource_man.load_fonts(&dir).unwrap();
            resource_man.load_scripts(&dir, namespace).unwrap();
            resource_man.load_researches(&dir, namespace).unwrap();

            log::info!("Loaded namespace {namespace}.");
        });

    resource_man
        .engine
        .definitions()
        .with_headers(true)
        .include_standard_packages(false)
        .write_to_dir("rhai")
        .unwrap();

    resource_man.ordered_tiles();
    resource_man.ordered_items();
    resource_man.compile_researches();
    resource_man.compile_categories();

    let resource_man = Arc::new(resource_man);

    resources::global::set_resource_man(resource_man.clone());

    resource_man
}
