use crate::game::game::{GameMsg, PlaceTileResponse};
use crate::game::input;
use crate::game::input::InputState;
use crate::game::map::{Map, MapRenderInfo, RenderContext};
use crate::game::setup::GameSetup;
use crate::game::tile::{TileCoord, TileEntityMsg, TileUnit};
use crate::render::camera::{cursor_to_pos, Camera};
use crate::render::data::InstanceData;
use crate::render::gpu::Gpu;
use crate::render::renderer::Renderer;
use crate::render::{gpu, gui};
use crate::util::cg::Num;
use crate::util::colors::Color;
use crate::util::id::Id;
use crate::util::resource::{ResourceManager, TileType};
use cgmath::{point2, EuclideanSpace};
use discord_rich_presence::activity::Timestamps;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use egui::style::Margin;
use egui::{Frame, Window};
use egui_winit_vulkano::Gui;
use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures_executor::block_on;
use hexagon_tiles::hex::Hex;
use kira::manager::AudioManager;
use riker::actor::{ActorRef, Tell};
use riker::actors::ActorSystem;
use riker_patterns::ask::ask;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

pub struct PersistentEventStorage {
    fuse: Fuse,
    closed: bool,
    script_filter: String,
    input_state: InputState,
    pointing_at: TileCoord,
    selected_tile_states: HashMap<Id, usize>,
    selected_id: Option<Id>,
    already_placed_at: Option<TileCoord>,
    config_open: Option<TileCoord>,
}
impl Default for PersistentEventStorage {
    fn default() -> Self {
        Self {
            fuse: Default::default(),
            closed: false,
            script_filter: String::new(),
            input_state: InputState::default(),
            pointing_at: TileCoord::ZERO,
            selected_tile_states: HashMap::<Id, usize>::new(),
            selected_id: None,
            already_placed_at: None,
            config_open: None,
        }
    }
}

pub fn on_event(
    setup: &mut GameSetup,
    persistent: &mut PersistentEventStorage,
    event: Event<()>,
    control_flow: &mut ControlFlow,
) -> Result<(), Box<dyn Error>> {
    if persistent.closed {
        return Ok(());
    }

    let mut window_event = None;
    let mut device_event = None;

    let mut gui_instances = vec![];
    let extra_vertices = vec![];

    let resource_man = setup.resource_man.clone();

    match &event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            block_on(setup.sys.shutdown())?;
            setup.discord_client.as_mut().map(|client| client.close());

            *control_flow = ControlFlow::Exit;

            persistent.closed = true;

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
                .update_pointing_at(persistent.input_state.main_pos);
            setup.renderer.gpu.window.request_redraw();
        }

        _ => {}
    };

    if window_event.is_some() || device_event.is_some() {
        persistent.input_state.reset();
        persistent
            .input_state
            .update(input::convert_input(window_event, device_event));

        let ignore_move = persistent.selected_id.is_some();

        setup
            .camera
            .input_state(persistent.input_state, ignore_move);

        persistent.pointing_at = setup.camera.camera_state().pointing_at;

        if persistent.input_state.exit_pressed {
            persistent.selected_id = None;
        }

        if persistent.input_state.main_pressed
            || (persistent.input_state.shift_held && persistent.input_state.main_held)
        {
            if let Some(id) = persistent.selected_id {
                if persistent.already_placed_at != Some(persistent.pointing_at) {
                    let response: PlaceTileResponse = block_on(ask(
                        &setup.sys,
                        &setup.game,
                        GameMsg::PlaceTile {
                            coord: persistent.pointing_at,
                            id,
                            none: setup.resource_man.none,
                            tile_state: *persistent.selected_tile_states.get(&id).unwrap_or(&0),
                        },
                    ));

                    if let PlaceTileResponse::Placed = response {
                        setup
                            .audio_man
                            .play(resource_man.audio["place"].clone())
                            .unwrap();
                    }

                    persistent.already_placed_at = Some(persistent.pointing_at)
                }
            }
        }

        if persistent.input_state.alternate_pressed {
            if let Some(id) = persistent.selected_id {
                let new = persistent.selected_tile_states.get(&id).unwrap_or(&0) + 1;

                persistent
                    .selected_tile_states
                    .insert(id, new % resource_man.tiles[&id].faces_indices.len());

                persistent.already_placed_at = None;
            }
            if persistent.config_open == Some(persistent.pointing_at) {
                persistent.config_open = None;
                persistent.script_filter.clear();
            } else {
                persistent.config_open = Some(persistent.pointing_at);
            }
        }
    }

    if event == Event::RedrawRequested(setup.renderer.gpu.window.id()) {
        let (selection_send, mut selection_recv) = mpsc::channel(1);

        setup.gui.immediate_ui(|gui| {
            // tile_selections
            gui::tile_selections(
                gui,
                resource_man.clone(),
                &persistent.selected_tile_states,
                &setup.renderer,
                selection_send,
            );

            // tile_info
            gui::tile_info(
                gui,
                resource_man.clone(),
                &setup.sys,
                setup.game.clone(),
                persistent.pointing_at,
            );

            if let Some(config_open) = persistent.config_open {
                let result: Option<(Id, ActorRef<TileEntityMsg>, usize)> =
                    block_on(ask(&setup.sys, &setup.game, GameMsg::GetTile(config_open)));

                if let Some((id, tile, _)) = result {
                    let current_script: Option<Id> =
                        block_on(ask(&setup.sys, &tile, TileEntityMsg::GetScript));
                    let mut new_script = current_script;

                    let current_target_coord: Option<Hex<TileUnit>> =
                        block_on(ask(&setup.sys, &tile, TileEntityMsg::GetTarget));
                    let mut new_target_coord = current_target_coord;

                    // tile_config
                    if let TileType::Machine(_) = resource_man.tiles[&id].tile_type {
                        if let Some(scripts) = resource_man.tiles[&id].scripts.clone() {
                            Window::new("Config")
                                .resizable(false)
                                .auto_sized()
                                .constrain(true)
                                .frame(setup.frame.inner_margin(Margin::same(10.0)))
                                .show(&gui.context(), |ui| {
                                    ui.set_max_width(300.0);

                                    let script_text = resource_man.try_item_name(&new_script);

                                    ui.label(format!("Script: {script_text}"));
                                    gui::scripts(
                                        ui,
                                        resource_man.clone(),
                                        &persistent.fuse,
                                        scripts,
                                        &mut new_script,
                                        &mut persistent.script_filter,
                                    );

                                    ui.separator();

                                    ui.label("Target:");
                                    gui::targets(ui, &mut new_target_coord);
                                });
                        }
                    }

                    if new_script != current_script {
                        if let Some(script) = new_script {
                            tile.tell(TileEntityMsg::SetScript(script), None);
                        }
                    }

                    if new_target_coord != current_target_coord {
                        setup.game.send_msg(
                            GameMsg::SendMsgToTile(
                                config_open,
                                TileEntityMsg::SetTarget(new_target_coord),
                            ),
                            None,
                        );
                    }
                }
            }
        });

        if let Ok(Some(id)) = selection_recv.try_next() {
            persistent.already_placed_at = None;

            if persistent.selected_id.is_some_and(|v| v == id) {
                persistent.selected_id = None;
            } else {
                persistent.selected_id = Some(id);
            }
        }

        let mouse_pos = cursor_to_pos(
            setup.camera.window_size.0,
            setup.camera.window_size.1,
            persistent.input_state.main_pos,
        );
        let mouse_pos = point2(mouse_pos.x, mouse_pos.y);
        let mouse_pos = mouse_pos + setup.camera.camera_state().pos.to_vec().truncate();

        let camera_state = setup.camera.camera_state();
        if camera_state.is_at_max_height() {
            if let Some(id) = persistent.selected_id {
                if let Some(faces_index) = resource_man.tiles.get(&id).and_then(|v| {
                    v.faces_indices
                        .get(*persistent.selected_tile_states.get(&id).unwrap_or(&0))
                }) {
                    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

                    let glow = (time.as_secs_f64() * 3.0).sin() / 10.0;

                    let instance = InstanceData::new()
                        .faces_index(*faces_index)
                        .position_offset([mouse_pos.x as Num, mouse_pos.y as Num, 0.1])
                        .color_offset(Color::TRANSPARENT.with_alpha(glow as Num).into());

                    gui_instances.push(instance);
                }
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
            setup.camera.camera_state(),
            resource_man.none,
            gui_instances,
            extra_vertices,
            &mut setup.gui,
        );
    }
    Ok(())
}
