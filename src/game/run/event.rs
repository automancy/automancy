use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use cgmath::{point2, EuclideanSpace};
use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures_executor::block_on;
use riker::actors::ActorRef;
use riker_patterns::ask::ask;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

use crate::game::input::InputState;
use crate::game::map::{Map, MapRenderInfo, RenderContext};
use crate::game::run::setup::GameSetup;
use crate::game::tile::coord::{ChunkCoord, TileCoord};
use crate::game::tile::entity::{Data, TileEntityMsg, TileState};
use crate::game::{input, GameMsg, PlaceTileResponse};
use crate::render::camera::{hex_to_normalized, screen_to_normalized, screen_to_world};
use crate::render::data::InstanceData;
use crate::render::renderer::RENDER_RANGE;
use crate::render::{gpu, gui};
use crate::resource::item::Item;
use crate::util::cg::{DPoint3, Num};
use crate::util::colors;
use crate::util::colors::WithAlpha;
use crate::util::id::Id;

/// Stores information that lives for the entire lifetime of the session, and is not dropped at the end of one event cycle or handled elsewhere.
#[derive(Default)]
pub struct EventLoopStorage {
    /// fuzzy search engine
    pub fuse: Fuse,
    /// whether or not the game should close.
    pub closed: bool,
    // TODO most of the following elements should be moved out of here...
    /// the filter for the scripts.
    pub filter: String,
    /// the state of the input peripherals.
    pub input_state: InputState,
    /// the tile states of the selected tiles.
    pub selected_tile_states: HashMap<Id, TileState>,
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
    /// the last camera position, in populate coord
    pub last_camera_chunk_coord: Option<ChunkCoord>,
}

/// Triggers every time the event loop is run once.
pub fn on_event(
    setup: &mut GameSetup,
    loop_store: &mut EventLoopStorage,
    event: Event<()>,
    control_flow: &mut ControlFlow,
) -> Result<(), Box<dyn Error>> {
    if loop_store.closed {
        return Ok(());
    }

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
            setup.game.send_msg(GameMsg::Stop, None);

            let map: Arc<Mutex<Map>> = block_on(ask(&setup.sys, &setup.game, GameMsg::GetMap));
            map.lock()
                .unwrap()
                .save(&setup.sys, &setup.resource_man.interner);

            block_on(setup.sys.shutdown())?;

            *control_flow = ControlFlow::Exit;

            loop_store.closed = true;

            return Ok(());
        }

        Event::WindowEvent { event, .. } => {
            if !setup.gui.update(event) {
                window_event = Some(event);
            }

            match event {
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    setup.renderer.recreate_swapchain = true;
                    setup.camera.window_size = gpu::window_size(&setup.renderer.gpu.window);
                }
                _ => {}
            }
        }

        Event::DeviceEvent { event, .. } => {
            device_event = Some(event);
        }

        Event::MainEventsCleared => {
            setup.camera.update_pos();
            setup
                .camera
                .update_pointing_at(loop_store.input_state.main_pos);
            setup.renderer.gpu.window.request_redraw();
        }

        _ => {}
    };

    if window_event.is_some() || device_event.is_some() {
        loop_store.input_state.reset();
        loop_store
            .input_state
            .update(input::convert_input(window_event, device_event));

        let ignore_move = loop_store.selected_id.is_some();

        setup
            .camera
            .input_state(loop_store.input_state, ignore_move);

        {
            let camera_chunk_coord = setup.camera.get_tile_coord().into();

            if Some(camera_chunk_coord) != loop_store.last_camera_chunk_coord {
                loop_store.last_camera_chunk_coord = Some(camera_chunk_coord);

                setup
                    .game
                    .send_msg(GameMsg::Populate(camera_chunk_coord), None);

                for coord in camera_chunk_coord.neighbors() {
                    setup.game.send_msg(GameMsg::Populate(coord), None);
                }
            }
        }

        if loop_store.input_state.exit_pressed {
            // cancel one by one
            if loop_store.selected_id.take().is_none() {
                loop_store.linking_tile.take();
            }
        }

        if loop_store.input_state.main_pressed
            || (loop_store.input_state.shift_held && loop_store.input_state.main_held)
        {
            if let Some(id) = loop_store.selected_id {
                if loop_store.already_placed_at != Some(setup.camera.camera_state().pointing_at) {
                    let response: PlaceTileResponse = block_on(ask(
                        &setup.sys,
                        &setup.game,
                        GameMsg::PlaceTile {
                            coord: setup.camera.camera_state().pointing_at,
                            id,
                            tile_state: *loop_store.selected_tile_states.get(&id).unwrap_or(&0),
                        },
                    ));

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

                    loop_store.already_placed_at = Some(setup.camera.camera_state().pointing_at)
                }
            }
        }

        if loop_store.input_state.alternate_pressed {
            if let Some(linking_tile) = loop_store.linking_tile {
                let result: Option<(ActorRef<TileEntityMsg>, Id, TileState)> = block_on(ask(
                    &setup.sys,
                    &setup.game,
                    GameMsg::GetTile(setup.camera.camera_state().pointing_at),
                ));

                if let Some(id) = result.map(|v| v.1) {
                    if id == resource_man.registry.tile_ids.node {
                        let old: Option<Data> = block_on(ask(
                            &setup.sys,
                            &setup.game,
                            GameMsg::SendMsgToTile(
                                setup.camera.camera_state().pointing_at,
                                TileEntityMsg::GetDataValue("link".to_string()),
                            ),
                        ));

                        if old.is_some() {
                            setup.game.send_msg(
                                GameMsg::SendMsgToTile(
                                    setup.camera.camera_state().pointing_at,
                                    TileEntityMsg::RemoveData("link".to_string()),
                                ),
                                None,
                            );
                            setup
                                .audio_man
                                .play(
                                    resource_man.audio["click"]
                                        .clone()
                                        .with_modified_settings(|s| s.playback_rate(0.5)),
                                )
                                .unwrap();
                        } else {
                            setup.game.send_msg(
                                GameMsg::SendMsgToTile(
                                    setup.camera.camera_state().pointing_at,
                                    TileEntityMsg::SetData(
                                        "link".to_string(),
                                        Data::Coord(
                                            linking_tile - setup.camera.camera_state().pointing_at,
                                        ),
                                    ),
                                ),
                                None,
                            );
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
                let new = loop_store.selected_tile_states.get(&id).unwrap_or(&0) + 1;
                let max = resource_man.registry.get_tile(id).unwrap().models.len() as i32;

                loop_store.selected_tile_states.insert(id, new % max);
                loop_store.already_placed_at = None;

                setup
                    .audio_man
                    .play(resource_man.audio["click"].clone())
                    .unwrap();
            } else if loop_store.config_open == Some(setup.camera.camera_state().pointing_at) {
                loop_store.config_open = None;
                loop_store.filter.clear();
            } else {
                loop_store.config_open = Some(setup.camera.camera_state().pointing_at);
            }
        }
    }

    if event == Event::RedrawRequested(setup.renderer.gpu.window.id()) {
        let (selection_send, mut selection_recv) = mpsc::channel(1);

        if setup.camera.is_at_max_height() {
            setup.gui.immediate_ui(|gui| {
                // tile_selections
                gui::tile_selections(
                    gui,
                    resource_man.clone(),
                    &loop_store.selected_tile_states,
                    &setup.renderer,
                    selection_send,
                );

                // tile_info
                gui::tile_info(
                    gui,
                    resource_man.clone(),
                    &setup.sys,
                    setup.game.clone(),
                    setup.camera.camera_state().pointing_at,
                );

                // tile_config
                gui::tile_config(
                    gui,
                    resource_man.clone(),
                    loop_store,
                    &mut extra_vertices,
                    &setup.camera,
                    &setup.sys,
                    setup.game.clone(),
                    setup.frame,
                );
            });
        }

        if let Ok(Some(id)) = selection_recv.try_next() {
            loop_store.already_placed_at = None;

            if loop_store.selected_id.is_some_and(|v| v == id) {
                loop_store.selected_id = None;
            } else {
                loop_store.selected_id = Some(id);
            }
        }

        let mouse_pos = screen_to_world(
            setup.camera.window_size.0,
            setup.camera.window_size.1,
            loop_store.input_state.main_pos,
        );
        let mouse_pos = point2(mouse_pos.x, mouse_pos.y);
        let mouse_pos = mouse_pos + setup.camera.get_pos().to_vec().truncate();

        if setup.camera.is_at_max_height() {
            if let Some(id) = loop_store.selected_id {
                if let Some(model) = resource_man.registry.get_tile(id).and_then(|v| {
                    v.models
                        .get(
                            loop_store
                                .selected_tile_states
                                .get(&id)
                                .cloned()
                                .unwrap_or(0) as usize,
                        )
                        .cloned()
                }) {
                    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

                    let glow = (time.as_secs_f64() * 3.0).sin() / 10.0;

                    let instance = InstanceData::new()
                        .model(model)
                        .position_offset([mouse_pos.x as Num, mouse_pos.y as Num, 0.1])
                        .color_offset(colors::TRANSPARENT.with_alpha(glow as Num).to_array());

                    gui_instances.push(instance);
                }
            }

            if let Some(coord) = loop_store.linking_tile {
                let DPoint3 { x, y, .. } = hex_to_normalized(
                    setup.camera.window_size.0,
                    setup.camera.window_size.1,
                    setup.camera.get_pos(),
                    coord,
                );
                let a = point2(x, y);

                let b = screen_to_normalized(
                    setup.camera.window_size.0,
                    setup.camera.window_size.1,
                    loop_store.input_state.main_pos,
                );

                extra_vertices.append(&mut gui::line(a, b, colors::RED));
            }
        }

        let render_info: Arc<MapRenderInfo> = block_on(ask(
            &setup.sys,
            &setup.game,
            GameMsg::RenderInfoRequest {
                context: RenderContext {
                    resource_man: resource_man.clone(),
                    range: RENDER_RANGE,
                    center: setup.camera.get_tile_coord(),
                },
            },
        ));

        setup.renderer.render(
            &render_info,
            setup.camera.get_pos(),
            setup.camera.camera_state().pointing_at,
            gui_instances,
            extra_vertices,
            &mut setup.gui,
        );
    }
    Ok(())
}
