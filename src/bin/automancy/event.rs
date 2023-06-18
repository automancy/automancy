use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use automancy_defs::cg::{DPoint3, Double, Float, Matrix4};
use automancy_defs::cgmath::{point2, vec3, EuclideanSpace};
use automancy_defs::colors::WithAlpha;
use automancy_defs::coord::{ChunkCoord, TileCoord, CHUNK_SIZE_SQUARED};
use automancy_defs::egui_winit_vulkano::Gui;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::Id;
use automancy_defs::rendering::InstanceData;
use automancy_defs::winit::event::{Event, WindowEvent};
use automancy_defs::winit::event_loop::ControlFlow;
use automancy_defs::{colors, log};
use automancy_resources::data::item::Item;
use automancy_resources::data::Data;
use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures::executor::block_on;
use tokio::runtime::Runtime;

use automancy::game::state::{GameMsg, PlaceTileResponse};
use automancy::game::tile::entity::{TileEntityMsg, TileModifier};
use automancy::render::camera::{hex_to_normalized, screen_to_normalized, screen_to_world, FAR};
use automancy::render::gui::{GuiState, PopupState};
use automancy::render::input::InputHandler;
use automancy::render::renderer::Renderer;
use automancy::render::{gui, input};

use crate::setup::GameSetup;

/// Stores information that lives for the entire lifetime of the session, and is not dropped at the end of one event cycle or handled elsewhere.
pub struct EventLoopStorage {
    /// fuzzy search engine
    pub fuse: Fuse,
    // TODO most of the following elements should be moved out of here...
    /// the filter for the scripts.
    pub filter: String,
    /// the state of the input peripherals.
    pub input_handler: InputHandler,
    /// the tile states of the selected tiles.
    pub selected_tile_modifiers: HashMap<Id, TileModifier>,
    /// the currently selected tile.
    pub selected_id: Option<Id>,
    /// the last placed tile, to prevent repeatedly sending place requests
    pub already_placed_at: Option<TileCoord>,
    /// the tile that has its config menu open.
    pub config_open: Option<TileCoord>,
    /// tag searching cache
    pub tag_cache: HashMap<Id, Arc<Vec<Item>>>,
    /// tile currently linking
    pub linking_tile: Option<TileCoord>,
    /// the last camera position, in chunk coord
    pub last_frame_start: Instant,
    /// the elapsed time between each frame
    pub elapsed: Duration,

    pub gui_state: GuiState,
    pub popup_state: PopupState,
}

impl Default for EventLoopStorage {
    fn default() -> Self {
        Self {
            fuse: Default::default(),
            filter: "".to_string(),
            input_handler: Default::default(),
            selected_tile_modifiers: Default::default(),
            selected_id: None,
            already_placed_at: None,
            config_open: None,
            tag_cache: Default::default(),
            linking_tile: None,
            last_frame_start: Instant::now(),
            elapsed: Default::default(),

            gui_state: GuiState::MainMenu,
            popup_state: PopupState::None,
        }
    }
}

pub fn shutdown_graceful(
    setup: &mut GameSetup,
    control_flow: &mut ControlFlow,
) -> Result<(), Box<dyn Error>> {
    setup.game.send_message(GameMsg::StopTicking).unwrap();

    block_on(setup.game.call(
        |reply| GameMsg::SaveMap(setup.resource_man.clone(), reply),
        None,
    ))
    .unwrap();

    setup.game.stop(Some("Game closed".to_string()));

    block_on(setup.game_handle.take().unwrap())?;

    control_flow.set_exit();

    log::info!("Shut down gracefully");
    Ok(())
}
/// Triggers every time the event loop is run once.
pub fn on_event(
    runtime: &Runtime,
    setup: &mut GameSetup,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
    gui: &mut Gui,
    event: Event<()>,
    control_flow: &mut ControlFlow,
) -> Result<(), Box<dyn Error>> {
    let window_size = setup.window.inner_size();

    let mut window_event = None;
    let mut device_event = None;

    let mut gui_instances = vec![];
    let mut extra_vertices = vec![];

    let resource_man = setup.resource_man.clone();

    match &event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            // game shutdown
            shutdown_graceful(setup, control_flow)?;
        }

        Event::WindowEvent { event, .. } => {
            if !gui.update(event) {
                window_event = Some(event);
            }

            match event {
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    renderer.recreate_swapchain = true;
                }
                _ => {}
            }
        }

        Event::DeviceEvent { event, .. } => {
            device_event = Some(event);
        }

        Event::MainEventsCleared => {
            renderer.gpu.window.request_redraw();
        }

        _ => {}
    };

    if *control_flow == ControlFlow::Exit {
        return Ok(());
    }

    if window_event.is_some() || device_event.is_some() {
        loop_store.input_handler.reset();
        loop_store
            .input_handler
            .update(input::convert_input(window_event, device_event));

        let ignore_move = loop_store.selected_id.is_some();

        setup
            .camera
            .input_handler(loop_store.input_handler, ignore_move);

        {
            let camera_chunk_coord: ChunkCoord = setup.camera.get_tile_coord().into();

            if setup.camera_chunk_coord != camera_chunk_coord {
                setup.camera_chunk_coord = camera_chunk_coord;

                // camera chunk coord update logic ...
            }
        }

        if loop_store.input_handler.exit_pressed {
            // cancel one by one
            if loop_store.selected_id.take().is_none() {
                loop_store.linking_tile.take();
            }
        }
        if loop_store.input_handler.main_pressed
            || (loop_store.input_handler.shift_held && loop_store.input_handler.main_held)
        {
            if let Some(id) = loop_store.selected_id {
                if loop_store.already_placed_at != Some(setup.camera.pointing_at) {
                    let response = runtime
                        .block_on(setup.game.call(
                            |reply| GameMsg::PlaceTile {
                                coord: setup.camera.pointing_at,
                                id,
                                tile_modifier:
                                    *loop_store.selected_tile_modifiers.get(&id).unwrap_or(&0),
                                record: true,
                                reply: Some(reply),
                            },
                            None,
                        ))
                        .unwrap()
                        .unwrap();

                    match response {
                        PlaceTileResponse::Placed => {
                            setup
                                .audio_man
                                .play(resource_man.audio["place"].clone())
                                .unwrap();
                        }
                        PlaceTileResponse::Removed => {
                            setup
                                .audio_man
                                .play(
                                    resource_man.audio["click"]
                                        .clone()
                                        .with_modified_settings(|s| s.playback_rate(0.5)),
                                )
                                .unwrap();
                        }
                        _ => {}
                    }

                    loop_store.already_placed_at = Some(setup.camera.pointing_at)
                }
            }
        }

        if loop_store.input_handler.alternate_pressed {
            if let Some(linking_tile) = loop_store.linking_tile {
                let tile = runtime
                    .block_on(setup.game.call(
                        |reply| GameMsg::GetTile(setup.camera.pointing_at, reply),
                        None,
                    ))
                    .unwrap()
                    .unwrap();

                let tile_entity = runtime
                    .block_on(setup.game.call(
                        |reply| GameMsg::GetTileEntity(setup.camera.pointing_at, reply),
                        None,
                    ))
                    .unwrap()
                    .unwrap();

                if let Some(((id, _), tile_entity)) = tile.zip(tile_entity) {
                    if id == resource_man.registry.tile_ids.node {
                        let old = runtime
                            .block_on(tile_entity.call(
                                |reply| {
                                    TileEntityMsg::GetDataValue(
                                        resource_man.registry.data_ids.link,
                                        reply,
                                    )
                                },
                                None,
                            ))
                            .unwrap()
                            .unwrap();

                        if old.is_some() {
                            tile_entity
                                .send_message(TileEntityMsg::RemoveData(
                                    resource_man.registry.data_ids.link,
                                ))
                                .unwrap();

                            setup
                                .audio_man
                                .play(
                                    resource_man.audio["click"]
                                        .clone()
                                        .with_modified_settings(|s| s.playback_rate(0.5)),
                                )
                                .unwrap();
                        } else {
                            tile_entity
                                .send_message(TileEntityMsg::SetData(
                                    resource_man.registry.data_ids.link,
                                    Data::Coord(linking_tile - setup.camera.pointing_at),
                                ))
                                .unwrap();

                            setup
                                .audio_man
                                .play(
                                    resource_man.audio["click"]
                                        .clone()
                                        .with_modified_settings(|s| s.playback_rate(1.5)),
                                )
                                .unwrap();
                            loop_store.linking_tile = None;
                        }
                    }
                }
            } else if let Some(id) = loop_store.selected_id {
                let new = loop_store.selected_tile_modifiers.get(&id).unwrap_or(&0) + 1;
                let max = resource_man.registry.tile(id).unwrap().models.len() as i32;

                loop_store.selected_tile_modifiers.insert(id, new % max);
                loop_store.already_placed_at = None;

                setup
                    .audio_man
                    .play(resource_man.audio["click"].clone())
                    .unwrap();
            } else if loop_store.config_open == Some(setup.camera.pointing_at) {
                loop_store.config_open = None;
                loop_store.filter.clear();
            } else {
                loop_store.config_open = Some(setup.camera.pointing_at);
            }
        }
    }

    if loop_store.input_handler.control_held && loop_store.input_handler.undo_pressed {
        setup.game.send_message(GameMsg::Undo).unwrap();
    }

    if event == Event::RedrawRequested(renderer.gpu.window.id()) {
        loop_store.last_frame_start = Instant::now();

        setup.camera.update_pos(loop_store.elapsed);
        setup.camera.update_pointing_at(
            loop_store.input_handler.main_pos,
            window_size.width as Double,
            window_size.height as Double,
        );

        let (selection_send, mut selection_recv) = mpsc::channel(1);

        gui.begin_frame();
        if loop_store.input_handler.pause_pressed {
            if loop_store.gui_state == GuiState::Ingame {
                block_on(setup.game.call(
                    |reply| GameMsg::SaveMap(setup.resource_man.clone(), reply),
                    None,
                ))
                .unwrap();
                loop_store.gui_state = GuiState::Paused;
            } else if loop_store.gui_state == GuiState::Paused {
                loop_store.gui_state = GuiState::Ingame;
            }
            loop_store.input_handler.pause_pressed = false;
        }
        match loop_store.gui_state {
            GuiState::MainMenu => gui::main_menu(setup, gui, control_flow, loop_store),
            GuiState::MapLoad => {
                gui::map_load_menu(setup, gui, loop_store, renderer);
            }
            GuiState::Options => {
                gui::options_menu(setup, gui, loop_store);
            }
            GuiState::Paused => {
                gui::pause_menu(setup, gui, loop_store, renderer);
            }
            GuiState::Ingame => {
                // tile_selections
                gui::tile_selections(
                    setup,
                    renderer,
                    gui,
                    &loop_store.selected_tile_modifiers,
                    selection_send,
                );

                // tile_info
                gui::tile_info(runtime, setup, gui);

                // tile_config
                gui::tile_config(
                    runtime,
                    setup,
                    loop_store,
                    renderer,
                    gui,
                    &mut extra_vertices,
                );

                if let Ok(Some(id)) = selection_recv.try_next() {
                    loop_store.already_placed_at = None;

                    if loop_store.selected_id == Some(id) {
                        loop_store.selected_id = None;
                    } else {
                        loop_store.selected_id = Some(id);
                    }
                }

                let mouse_pos = screen_to_world(
                    window_size.width as Double,
                    window_size.height as Double,
                    loop_store.input_handler.main_pos,
                    setup.camera.get_pos().z,
                );
                let mouse_pos = point2(mouse_pos.x, mouse_pos.y);
                let mouse_pos = mouse_pos + setup.camera.get_pos().to_vec().truncate();

                if let Some(id) = loop_store.selected_id {
                    if let Some(model) = resource_man.registry.tile(id).and_then(|v| {
                        v.models
                            .get(
                                loop_store
                                    .selected_tile_modifiers
                                    .get(&id)
                                    .cloned()
                                    .unwrap_or(0) as usize,
                            )
                            .cloned()
                    }) {
                        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

                        let glow = (time.as_secs_f64() * 3.0).sin() / 3.0;

                        let instance = InstanceData {
                            model_matrix: Matrix4::from_translation(vec3(
                                mouse_pos.x as Float,
                                mouse_pos.y as Float,
                                FAR as Float,
                            )),
                            color_offset: colors::TRANSPARENT.with_alpha(glow as Float).to_array(),
                        };

                        gui_instances.push((instance, model));
                    }
                }

                if let Some(coord) = loop_store.linking_tile {
                    let DPoint3 { x, y, .. } = hex_to_normalized(
                        window_size.width as Double,
                        window_size.height as Double,
                        setup.camera.get_pos(),
                        coord,
                    );
                    let a = point2(x, y);

                    let b = screen_to_normalized(
                        window_size.width as Double,
                        window_size.height as Double,
                        loop_store.input_handler.main_pos,
                    );

                    extra_vertices.append(&mut gui::line(a, b, colors::RED));
                }
            }
        }
        match &loop_store.popup_state {
            PopupState::None => {}
            PopupState::MapCreate => gui::map_create_menu(setup, gui, loop_store, renderer),
            PopupState::MapDeleteConfirmation(map) => {
                gui::map_delete_confirmation(setup, gui, loop_store, map.clone());
            }
        }
        if loop_store.input_handler.debug_pressed {
            gui::debugger(setup, gui, runtime, &setup.game, renderer, loop_store);
        }
        if resource_man.error_man.has_errors() {
            gui::error_popup(setup, gui);
        }
        let render_info = block_on(setup.game.call(
            |reply| GameMsg::RenderInfoRequest {
                range: CHUNK_SIZE_SQUARED,
                center: setup.camera.get_tile_coord(),
                reply,
            },
            None,
        ))?
        .unwrap();

        renderer.render(
            runtime,
            setup,
            &render_info,
            gui_instances,
            extra_vertices,
            gui,
        );
        loop_store.elapsed = Instant::now().duration_since(loop_store.last_frame_start);
    }
    Ok(())
}
