use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use cgmath::{point2, vec2, EuclideanSpace, MetricSpace};
use egui::style::Margin;
use egui::Window;
use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures_executor::block_on;
use genmesh::{EmitTriangles, Quad};
use riker::actor::{ActorRef, Tell};
use riker_patterns::ask::ask;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

use crate::game::input::InputState;
use crate::game::map::{Map, MapRenderInfo, RenderContext};
use crate::game::run::setup::GameSetup;
use crate::game::tile::coord::TileCoord;
use crate::game::tile::entity::{Data, DataMap, TileEntityMsg, TileState};
use crate::game::{input, GameMsg, PlaceTileResponse};
use crate::render::camera::{hex_to_normalized, screen_to_normalized, screen_to_world};
use crate::render::data::{InstanceData, Vertex};
use crate::render::{gpu, gui};
use crate::resource::item::Item;
use crate::resource::tile::TileType;
use crate::util::cg::{DPoint3, Num};
use crate::util::colors::WithAlpha;
use crate::util::id::Id;
use crate::util::{colors, format};

/// Stores information that lives for the entire lifetime of the session, and is not dropped at the end of one event cycle or handled elsewhere.
pub struct EventLoopStorage {
    /// fuzzy search engine
    fuse: Fuse,
    /// whether or not the game should close.
    closed: bool,
    // TODO most of the following elements should be moved out of here...
    /// the filter for the scripts.
    filter: String,
    /// the state of the input peripherals.
    input_state: InputState,
    /// the tile the cursor is pointing at.
    pointing_at: TileCoord,
    /// the tile states of the selected tiles.
    selected_tile_states: HashMap<Id, TileState>,
    /// the currently selected tile.
    selected_id: Option<Id>,
    /// the last placed tile, to prevent repeatedly sending place requests
    already_placed_at: Option<TileCoord>,
    /// the tile that has its config menu open.
    config_open: Option<TileCoord>,
    /// tag searching cache
    tag_cache: HashMap<Id, Arc<Vec<Item>>>,
    /// tile currently linking
    linking_tile: Option<TileCoord>,
}

impl Default for EventLoopStorage {
    fn default() -> Self {
        Self {
            fuse: Default::default(),
            closed: false,
            filter: String::new(),
            input_state: Default::default(),
            pointing_at: TileCoord::ZERO,
            selected_tile_states: Default::default(),
            selected_id: None,
            already_placed_at: None,
            config_open: None,
            tag_cache: Default::default(),
            linking_tile: None,
        }
    }
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

        loop_store.pointing_at = setup.camera.camera_state().pointing_at;

        if loop_store.input_state.exit_pressed {
            loop_store.selected_id = None;
        }

        if loop_store.input_state.main_pressed
            || (loop_store.input_state.shift_held && loop_store.input_state.main_held)
        {
            if let Some(id) = loop_store.selected_id {
                if loop_store.already_placed_at != Some(loop_store.pointing_at) {
                    let response: PlaceTileResponse = block_on(ask(
                        &setup.sys,
                        &setup.game,
                        GameMsg::PlaceTile {
                            coord: loop_store.pointing_at,
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

                    loop_store.already_placed_at = Some(loop_store.pointing_at)
                }
            }
        }

        if loop_store.input_state.alternate_pressed {
            if let Some(id) = loop_store.selected_id {
                let new = loop_store.selected_tile_states.get(&id).unwrap_or(&0) + 1;
                let max = resource_man.registry.get_tile(id).unwrap().models.len() as i32;

                loop_store.selected_tile_states.insert(id, new % max);
                loop_store.already_placed_at = None;

                setup
                    .audio_man
                    .play(resource_man.audio["click"].clone())
                    .unwrap();
            } else if loop_store.config_open == Some(loop_store.pointing_at) {
                loop_store.config_open = None;
                loop_store.filter.clear();
            } else {
                loop_store.config_open = Some(loop_store.pointing_at);
            }
        }
    }

    if event == Event::RedrawRequested(setup.renderer.gpu.window.id()) {
        let camera_state = setup.camera.camera_state();

        let (selection_send, mut selection_recv) = mpsc::channel(1);

        if camera_state.is_at_max_height() {
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
                    loop_store.pointing_at,
                );

                if let Some(config_open) = loop_store.config_open {
                    let result: Option<(ActorRef<TileEntityMsg>, Id, TileState)> =
                        block_on(ask(&setup.sys, &setup.game, GameMsg::GetTile(config_open)));

                    if let Some((tile, id, _)) = result {
                        let data: DataMap =
                            block_on(ask(&setup.sys, &tile, TileEntityMsg::GetData));

                        let current_amount = data
                            .get("amount")
                            .and_then(Data::as_amount)
                            .cloned()
                            .unwrap_or(0);
                        let mut new_amount = current_amount;

                        let current_script = data.get("script").and_then(Data::as_id).cloned();
                        let mut new_script = current_script;

                        let current_storage = data.get("storage").and_then(Data::as_id).cloned();
                        let mut new_storage = current_storage;

                        let current_target_coord =
                            data.get("target").and_then(Data::as_coord).cloned();
                        let mut new_target_coord = current_target_coord;

                        // tile_config
                        Window::new(
                            resource_man.translates.gui[&resource_man.registry.gui_ids.tile_config]
                                .to_string(),
                        )
                        .resizable(false)
                        .auto_sized()
                        .constrain(true)
                        .frame(setup.frame.inner_margin(Margin::same(10.0)))
                        .show(&gui.context(), |ui| {
                            const MARGIN: Num = 8.0;

                            ui.set_max_width(300.0);

                            match &resource_man.registry.get_tile(id).unwrap().tile_type {
                                TileType::Machine(scripts) => {
                                    let script_text = if let Some(script) = new_script
                                        .and_then(|id| resource_man.registry.get_script(id))
                                    {
                                        let input = if let Some(input) = script.instructions.input {
                                            format!(
                                                "{} ({})",
                                                resource_man.item_name(&input.item.id),
                                                input.amount
                                            )
                                        } else {
                                            String::new()
                                        };

                                        let output =
                                            if let Some(output) = script.instructions.output {
                                                format!(
                                                    "=> {} ({})",
                                                    resource_man.item_name(&output.item.id),
                                                    output.amount
                                                )
                                            } else {
                                                String::new()
                                            };

                                        if !input.is_empty() && !output.is_empty() {
                                            format!("{input}\n{output}")
                                        } else {
                                            format!("{input}{output}")
                                        }
                                    } else {
                                        "<none>".to_string()
                                    };

                                    ui.add_space(MARGIN);

                                    ui.label(
                                        resource_man.translates.gui
                                            [&resource_man.registry.gui_ids.tile_config_script]
                                            .as_str(),
                                    );
                                    ui.label(script_text);

                                    ui.add_space(MARGIN);

                                    gui::searchable_id(
                                        ui,
                                        resource_man.clone(),
                                        &loop_store.fuse,
                                        scripts.as_slice(),
                                        &mut new_script,
                                        &mut loop_store.filter,
                                    );
                                }
                                TileType::Storage(storage) => {
                                    let storage_text = if let Some(item) = new_storage
                                        .and_then(|id| resource_man.registry.get_item(id))
                                    {
                                        resource_man.item_name(&item.id).to_string()
                                    } else {
                                        "<none>".to_string()
                                    };

                                    let items = resource_man
                                        .get_items(storage.id, &mut loop_store.tag_cache)
                                        .iter()
                                        .map(|item| item.id)
                                        .collect::<Vec<_>>();

                                    ui.add_space(MARGIN);

                                    ui.label(
                                        resource_man.translates.gui
                                            [&resource_man.registry.gui_ids.tile_config_storage]
                                            .as_str(),
                                    );
                                    ui.horizontal(|ui| {
                                        ui.label(storage_text);
                                        ui.add(
                                            egui::DragValue::new(&mut new_amount)
                                                .clamp_range(0..=65535)
                                                .speed(1.0)
                                                .prefix("Amount:"), // TODO translate
                                        );
                                    });

                                    ui.add_space(MARGIN);

                                    gui::searchable_id(
                                        ui,
                                        resource_man.clone(),
                                        &loop_store.fuse,
                                        items.as_slice(),
                                        &mut new_storage,
                                        &mut loop_store.filter,
                                    );
                                }
                                TileType::Transfer(id) => {
                                    if id == &resource_man.registry.tile_ids.inventory_linker {
                                        ui.add_space(MARGIN);

                                        if ui.button("Link Network!").clicked() {
                                            loop_store.linking_tile = Some(config_open);
                                        };
                                        ui.label("(Right click to link Destination)");

                                        ui.add_space(MARGIN);
                                    }
                                }
                                _ => {}
                            }

                            if resource_man.registry.get_tile(id).unwrap().targeted {
                                ui.add_space(MARGIN);

                                ui.label(format(
                                    &resource_man.translates.gui
                                        [&resource_man.registry.gui_ids.tile_config_target],
                                    &[],
                                ));
                                gui::targets(ui, &mut new_target_coord);
                            }

                            ui.add_space(MARGIN);
                        });

                        if new_amount != current_amount {
                            tile.tell(
                                TileEntityMsg::SetData(
                                    "amount".to_string(),
                                    Data::Amount(new_amount),
                                ),
                                None,
                            );
                        }

                        if new_script != current_script {
                            if let Some(script) = new_script {
                                tile.tell(
                                    TileEntityMsg::SetData("script".to_string(), Data::Id(script)),
                                    None,
                                );
                                tile.tell(TileEntityMsg::RemoveData("buffer".to_string()), None);
                            }
                        }

                        if new_storage != current_storage {
                            if let Some(storage) = new_storage {
                                tile.tell(
                                    TileEntityMsg::SetData(
                                        "storage".to_string(),
                                        Data::Id(storage),
                                    ),
                                    None,
                                );
                                tile.tell(TileEntityMsg::RemoveData("buffer".to_string()), None);
                            }
                        }

                        if new_target_coord != current_target_coord {
                            if let Some(target_coord) = new_target_coord {
                                setup.game.send_msg(
                                    GameMsg::SendMsgToTile(
                                        config_open,
                                        TileEntityMsg::SetData(
                                            "target".to_string(),
                                            Data::Coord(target_coord),
                                        ),
                                    ),
                                    None,
                                );
                            } else {
                                setup.game.send_msg(
                                    GameMsg::SendMsgToTile(
                                        config_open,
                                        TileEntityMsg::RemoveData("target".to_string()),
                                    ),
                                    None,
                                );
                            }
                        }
                    }
                }
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
        let mouse_pos = mouse_pos + camera_state.pos.to_vec().truncate();

        if camera_state.is_at_max_height() {
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
                    camera_state.pos,
                    coord,
                );
                let a = point2(x, y);

                let b = screen_to_normalized(
                    setup.camera.window_size.0,
                    setup.camera.window_size.1,
                    loop_store.input_state.main_pos,
                );

                let v = b - a;
                let l = a.distance(b) * 128.0;
                let w = vec2(-v.y / l, v.x / l);

                let a0 = (a + w).cast::<Num>().unwrap();
                let a1 = (b + w).cast::<Num>().unwrap();
                let b0 = (b - w).cast::<Num>().unwrap();
                let b1 = (a - w).cast::<Num>().unwrap();

                let mut line = vec![];

                Quad::new(
                    Vertex {
                        pos: [a0.x, a0.y, 0.0],
                        color: colors::ORANGE.to_array(),
                        normal: [0.0, 0.0, 0.0],
                    },
                    Vertex {
                        pos: [a1.x, a1.y, 0.0],
                        color: colors::ORANGE.to_array(),
                        normal: [0.0, 0.0, 0.0],
                    },
                    Vertex {
                        pos: [b0.x, b0.y, 0.0],
                        color: colors::ORANGE.to_array(),
                        normal: [0.0, 0.0, 0.0],
                    },
                    Vertex {
                        pos: [b1.x, b1.y, 0.0],
                        color: colors::ORANGE.to_array(),
                        normal: [0.0, 0.0, 0.0],
                    },
                )
                .emit_triangles(|v| line.append(&mut vec![v.x, v.y, v.z]));

                extra_vertices.append(&mut line);
            }
        }

        let render_info: MapRenderInfo = block_on(ask(
            &setup.sys,
            &setup.game,
            GameMsg::RenderInfoRequest {
                context: RenderContext {
                    resource_man: resource_man.clone(),
                },
            },
        ));

        setup.renderer.render(
            render_info,
            camera_state,
            gui_instances,
            extra_vertices,
            &mut setup.gui,
        );
    }
    Ok(())
}
