use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use cgmath::{point2, vec3, EuclideanSpace};
use egui_winit_vulkano::Gui;

use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures_executor::block_on;
use riker::actors::ActorRef;
use riker_patterns::ask::ask;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

use crate::game::input::InputHandler;
use crate::game::map::{Map, TileEntities};
use crate::game::run::setup::GameSetup;
use crate::game::tile::coord::{ChunkCoord, TileCoord};
use crate::game::tile::entity::{Data, TileEntityMsg, TileState};
use crate::game::{input, GameMsg, PlaceTileResponse, RenderInfo};
use crate::render::camera::{hex_to_normalized, screen_to_normalized, screen_to_world, FAR};
use crate::render::data::InstanceData;
use crate::render::gui;
use crate::render::renderer::{Renderer, RENDER_RANGE};
use crate::resource::item::Item;
use crate::util::cg::{DPoint3, Double, Float};
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
    pub input_handler: InputHandler,
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
    renderer: &mut Renderer,
    gui: &mut Gui,
    event: Event<()>,
    control_flow: &mut ControlFlow,
) -> Result<(), Box<dyn Error>> {
    if loop_store.closed {
        return Ok(());
    }

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
            setup.game.send_msg(GameMsg::Stop, None);

            let map: Map = block_on(ask(&setup.sys, &setup.game, GameMsg::TakeMap));

            let tile_entities: TileEntities =
                block_on(ask(&setup.sys, &setup.game, GameMsg::TakeTileEntities));

            map.save(&setup.sys, &setup.resource_man.interner, tile_entities);

            block_on(setup.sys.shutdown())?;

            *control_flow = ControlFlow::Exit;

            loop_store.closed = true;

            return Ok(());
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
            setup.camera.update_pos();
            setup.camera.update_pointing_at(
                loop_store.input_handler.main_pos,
                window_size.width as Double,
                window_size.height as Double,
            );
            renderer.gpu.window.request_redraw();
        }

        _ => {}
    };

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
                    let response: PlaceTileResponse = block_on(ask(
                        &setup.sys,
                        &setup.game,
                        GameMsg::PlaceTile {
                            coord: setup.camera.pointing_at,
                            id,
                            tile_state: *loop_store.selected_tile_states.get(&id).unwrap_or(&0),
                            record: true,
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

                    loop_store.already_placed_at = Some(setup.camera.pointing_at)
                }
            }
        }

        if loop_store.input_handler.alternate_pressed {
            if let Some(linking_tile) = loop_store.linking_tile {
                let result: Option<(ActorRef<TileEntityMsg>, Id, TileState)> = block_on(ask(
                    &setup.sys,
                    &setup.game,
                    GameMsg::GetTile(setup.camera.pointing_at),
                ));

                if let Some(id) = result.map(|v| v.1) {
                    if id == resource_man.registry.tile_ids.node {
                        let old: Option<Data> = block_on(ask(
                            &setup.sys,
                            &setup.game,
                            GameMsg::ForwardMsgToTile(
                                setup.camera.pointing_at,
                                TileEntityMsg::GetDataValue("link"),
                            ),
                        ));

                        if old.is_some() {
                            setup.game.send_msg(
                                GameMsg::ForwardMsgToTile(
                                    setup.camera.pointing_at,
                                    TileEntityMsg::RemoveData("link"),
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
                                GameMsg::ForwardMsgToTile(
                                    setup.camera.pointing_at,
                                    TileEntityMsg::SetData(
                                        "link".to_owned(),
                                        Data::Coord(linking_tile - setup.camera.pointing_at),
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
                let max = resource_man.registry.get_tile(&id).unwrap().models.len() as i32;

                loop_store.selected_tile_states.insert(id, new % max);
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
        setup.game.send_msg(GameMsg::Undo, None);
    }

    if event == Event::RedrawRequested(renderer.gpu.window.id()) {
        let (selection_send, mut selection_recv) = mpsc::channel(1);

        gui.begin_frame();

        // tile_selections
        gui::tile_selections(
            setup,
            renderer,
            gui,
            &loop_store.selected_tile_states,
            selection_send,
        );

        // tile_info
        gui::tile_info(setup, gui, setup.game.clone(), setup.camera.pointing_at);

        // tile_config
        gui::tile_config(
            setup,
            loop_store,
            gui,
            setup.game.clone(),
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
            if let Some(model) = resource_man.registry.get_tile(&id).and_then(|v| {
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

                let instance = InstanceData::default()
                    .add_translation(vec3(
                        mouse_pos.x as Float,
                        mouse_pos.y as Float,
                        FAR as Float,
                    ))
                    .with_color_offset(colors::TRANSPARENT.with_alpha(glow as Float).to_array());

                gui_instances.push((instance.into(), model));
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

        let render_info: Arc<RenderInfo> = block_on(ask(
            &setup.sys,
            &setup.game,
            GameMsg::RenderInfoRequest {
                range: RENDER_RANGE,
                center: setup.camera.get_tile_coord(),
            },
        ));

        renderer.render(setup, &render_info, gui_instances, extra_vertices, gui);
    }
    Ok(())
}
