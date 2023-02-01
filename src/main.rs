#![feature(result_option_inspect)]
#![feature(is_some_and)]

use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, sync::Arc};

use cgmath::{point3, EuclideanSpace};
use discord_rich_presence::DiscordIpc;
use egui::style::Margin;
use egui::{vec2, Align, Align2, ScrollArea, TopBottomPanel, Window};
use env_logger::Env;
use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures::executor::block_on;
use hexagon_tiles::hex::Hex;
use riker::actor::ActorRef;
use riker::actors::{ActorRefFactory, SystemBuilder, Tell, Timer};
use riker_patterns::ask::ask;
use vulkano::device::DeviceExtensions;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Icon,
};

use automancy::game::data::Data;
use automancy::game::game::GameMsg;
use automancy::game::input;
use automancy::game::input::InputState;
use automancy::game::map::{MapRenderInfo, RenderContext};
use automancy::game::tile::{TileCoord, TileMsg, TileUnit};
use automancy::render::data::InstanceData;
use automancy::render::gpu::{Gpu, RenderAlloc};
use automancy::render::gui;
use automancy::util::cg::Num;
use automancy::util::colors::Color;
use automancy::util::discord;
use automancy::util::id::Id;
use automancy::util::init::InitData;
use automancy::util::resource::{ResourceManager, ResourceType};
use automancy::{
    game::map::Map,
    game::{game::Game, ticking::TICK_INTERVAL},
    render::{camera::Camera, gpu, renderer::Renderer},
    LOGO, RESOURCE,
};

fn init_data() -> Arc<InitData> {
    let mut resource_man = ResourceManager::new();

    fs::read_dir(RESOURCE)
        .unwrap()
        .flatten()
        .map(|v| v.path())
        .for_each(|dir| {
            resource_man.load_models(&dir);
            resource_man.load_scripts(&dir);
            resource_man.load_translates(&dir);
            resource_man.load_tiles(&dir);
        });

    let init_data = InitData::new(resource_man);

    Arc::new(init_data)
}

fn get_icon() -> Icon {
    let (bytes, width, height) = {
        let decoder = png::Decoder::new(LOGO);

        let mut reader = decoder.read_info().unwrap();

        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();

        (buf[..info.buffer_size()].to_vec(), info.width, info.height)
    };

    Icon::from_rgba(bytes, width, height).unwrap()
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // --- resources & data ---
    let init_data = init_data();
    log::info!("loaded resources.");

    let icon = get_icon();

    // --- setup render ---
    let event_loop = EventLoop::new();

    let instance = gpu::create_instance();
    let window = gpu::create_window(icon, &event_loop);
    let surface = gpu::create_surface(window.clone(), instance.clone());

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        khr_dedicated_allocation: true,
        khr_get_memory_requirements2: true,
        ..DeviceExtensions::default()
    };

    let (physical_device, queue_family_index) =
        gpu::get_physical_device(instance.clone(), surface.clone(), &device_extensions);
    log::info!(
        "Using device: {} (type: {:?})",
        physical_device.clone().properties().device_name,
        physical_device.clone().properties().device_type
    );

    let (device, mut queues) = gpu::get_logical_device(
        physical_device.clone(),
        queue_family_index,
        device_extensions,
    );
    let queue = queues.next().unwrap();

    let alloc = RenderAlloc::new(
        init_data.clone(),
        device.clone(),
        queue.clone(),
        surface.clone(),
        window.clone(),
        physical_device,
    );
    let gpu = Gpu::new(device, queue, surface, window, alloc);

    let mut gui = gui::init_gui(&event_loop, &gpu);

    // --- setup game ---
    let sys = SystemBuilder::new().name("automancy").create().unwrap();

    //let map = Map::load("test".to_owned());
    let map = Map::new_empty("test".to_owned());

    let game = sys
        .actor_of_args::<Game, Arc<Map>>("game", Arc::new(map))
        .unwrap();

    sys.schedule(
        TICK_INTERVAL,
        TICK_INTERVAL,
        game.clone(),
        None,
        GameMsg::Tick {
            init_data: init_data.clone(),
        },
    );

    log::info!("loading completed!");

    // last setup
    let none = init_data.resource_man.none;
    let frame = gui::default_frame();

    let start_time = discord::start_time();
    let mut discord_client = discord::setup_rich_presence().ok();

    discord_client.as_mut().map(|client| {
        discord::set_status(client, start_time.clone(), discord::DiscordStatuses::InGame).unwrap()
    });

    let mut renderer = Renderer::new(init_data.clone(), gpu);
    let mut camera = Camera::new(gpu::window_size(&renderer.gpu.window));

    // --- event-loop ---
    {
        let fuse = Fuse::default();

        let mut closed = false;
        let mut script_filter = String::new();

        let mut input_state = InputState::default();

        let mut pointing_at = TileCoord::ZERO;

        let mut selected_id = None;
        let mut config_open = None;

        event_loop.run(move |event, _, control_flow| {
            if closed {
                return;
            }

            let mut window_event = None;
            let mut device_event = None;

            let mut extra_instances = vec![];
            let extra_vertices = vec![];

            let init_data = init_data.clone();

            match &event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    block_on(sys.shutdown()).unwrap();
                    discord_client.as_mut().map(|client| client.close());

                    *control_flow = ControlFlow::Exit;

                    closed = true;

                    return;
                }

                Event::WindowEvent { event, .. } => {
                    if !gui.update(&event) {
                        window_event = Some(event);
                    }

                    match event {
                        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                            renderer.recreate_swapchain = true;
                            camera.window_size = gpu::window_size(&renderer.gpu.window);
                        }
                        _ => {}
                    }
                }

                Event::DeviceEvent { event, .. } => {
                    device_event = Some(event);
                }

                Event::MainEventsCleared => {
                    camera.update_pos();
                    camera.update_pointing_at(input_state.main_pos);
                    renderer.gpu.window.request_redraw();
                }

                _ => {}
            };

            if window_event.is_some() || device_event.is_some() {
                input_state.reset();

                input_state.update(input::convert_input(window_event, device_event));

                let ignore_move = selected_id.is_some();

                camera.input_state(input_state, ignore_move);

                pointing_at = camera.camera_state().pointing_at;

                if input_state.exit_pressed {
                    selected_id = None;
                }

                if let Some(id) = selected_id {
                    if input_state.main_pressed {
                        game.tell(
                            GameMsg::PlaceTile {
                                coord: pointing_at,
                                id,
                                none,
                            },
                            None,
                        );
                    }
                }

                if input_state.alternate_pressed {
                    if config_open == Some(pointing_at) {
                        config_open = None;
                        script_filter.clear();
                    } else {
                        config_open = Some(pointing_at);
                    }
                }
            }

            if event == Event::RedrawRequested(renderer.gpu.window.id()) {
                let (selection_send, mut selection_recv) = mpsc::channel(1);

                gui.immediate_ui(|gui| {
                    // tile_selection
                    TopBottomPanel::bottom("tile_selection")
                        .show_separator_line(false)
                        .resizable(false)
                        .frame(frame.outer_margin(Margin::same(10.0)))
                        .show(&gui.context(), |ui| {
                            let spacing = ui.spacing_mut();

                            spacing.interact_size.y = 70.0;
                            spacing.scroll_bar_width = 0.0;
                            spacing.scroll_bar_outer_margin = 0.0;

                            ScrollArea::horizontal()
                                .always_show_scroll(true)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        gui::render_tile_selection(
                                            ui,
                                            init_data.clone(),
                                            &renderer.gpu,
                                            selection_send,
                                        );
                                    });
                                });
                        });

                    // tile_info
                    Window::new("Tile Info")
                        .anchor(Align2([Align::RIGHT, Align::TOP]), vec2(-10.0, 10.0))
                        .resizable(false)
                        .default_width(300.0)
                        .frame(frame.inner_margin(Margin::same(10.0)))
                        .show(&gui.context(), |ui| {
                            let coord = pointing_at;

                            ui.colored_label(Color::DARK_GRAY, coord.to_string());

                            let result: Option<(Id, ActorRef<TileMsg>)> =
                                block_on(ask(&sys, &game, GameMsg::GetTile(coord)));

                            if let Some((id, tile)) = result {
                                ui.label(init_data.resource_man.tile_name(&id));
                                let data: Data = block_on(ask(&sys, &tile, TileMsg::GetData));

                                for (id, amount) in data.0.iter() {
                                    ui.label(format!(
                                        "{} - {}",
                                        init_data.resource_man.item_name(id),
                                        amount
                                    ));
                                }
                            }
                        });

                    if let Some(config_open) = config_open {
                        let result: Option<(Id, ActorRef<TileMsg>)> =
                            block_on(ask(&sys, &game, GameMsg::GetTile(config_open)));

                        if let Some((id, tile)) = result {
                            let current_script: Option<Id> =
                                block_on(ask(&sys, &tile, TileMsg::GetScript));
                            let mut new_script = current_script.clone();

                            let current_target_coord: Option<Hex<TileUnit>> =
                                block_on(ask(&sys, &tile, TileMsg::GetTarget));
                            let mut new_target_coord = current_target_coord;

                            // tile_config
                            if let ResourceType::Machine(_) =
                                init_data.resource_man.resources[&id].resource_type
                            {
                                if let Some(scripts) =
                                    init_data.resource_man.resources[&id].scripts.clone()
                                {
                                    Window::new("Config")
                                        .resizable(false)
                                        .auto_sized()
                                        .constrain(true)
                                        .frame(frame.inner_margin(Margin::same(10.0)))
                                        .show(&gui.context(), |ui| {
                                            ui.set_max_width(300.0);

                                            let script_text = if let Some(ref script) = new_script {
                                                init_data.resource_man.item_name(script)
                                            // TODO move this to resource_man
                                            } else {
                                                "<none>".to_owned()
                                            };

                                            ui.label(format!("Script: {}", script_text));

                                            ui.text_edit_singleline(&mut script_filter);

                                            ScrollArea::vertical().max_height(80.0).show(
                                                ui,
                                                |ui| {
                                                    ui.set_width(ui.available_width());

                                                    let scripts = if !script_filter.is_empty() {
                                                        let mut filtered = scripts
                                                            .into_iter()
                                                            .flat_map(|id| {
                                                                let result = fuse
                                                                    .search_text_in_string(
                                                                        &script_filter,
                                                                        init_data
                                                                            .resource_man
                                                                            .item_name(&id)
                                                                            .as_str(),
                                                                    );

                                                                Some(id)
                                                                    .zip(result.map(|v| v.score))
                                                            })
                                                            .collect::<Vec<_>>();

                                                        filtered.sort_unstable_by(|a, b| {
                                                            a.1.total_cmp(&b.1)
                                                        });

                                                        filtered
                                                            .into_iter()
                                                            .map(|v| v.0)
                                                            .collect::<Vec<_>>()
                                                    } else {
                                                        scripts
                                                    };

                                                    scripts.iter().for_each(|script| {
                                                        ui.radio_value(
                                                            &mut new_script,
                                                            Some(*script),
                                                            init_data
                                                                .resource_man
                                                                .item_name(&script),
                                                        );
                                                    })
                                                },
                                            );

                                            ui.separator();

                                            ui.label("Target:");
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(15.0);
                                                    gui::add_direction(
                                                        ui,
                                                        &mut new_target_coord,
                                                        5,
                                                    );
                                                    gui::add_direction(
                                                        ui,
                                                        &mut new_target_coord,
                                                        0,
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    gui::add_direction(
                                                        ui,
                                                        &mut new_target_coord,
                                                        4,
                                                    );
                                                    ui.selectable_value(
                                                        &mut new_target_coord,
                                                        None,
                                                        "❌",
                                                    );
                                                    gui::add_direction(
                                                        ui,
                                                        &mut new_target_coord,
                                                        1,
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.add_space(15.0);
                                                    gui::add_direction(
                                                        ui,
                                                        &mut new_target_coord,
                                                        3,
                                                    );
                                                    gui::add_direction(
                                                        ui,
                                                        &mut new_target_coord,
                                                        2,
                                                    );
                                                });
                                            });
                                        });
                                }
                            }

                            if new_script != current_script {
                                if let Some(script) = new_script {
                                    tile.tell(TileMsg::SetScript(script), None);
                                }
                            }

                            if new_target_coord != current_target_coord {
                                game.send_msg(
                                    GameMsg::SendMsgToTile(
                                        config_open,
                                        TileMsg::SetTarget(new_target_coord),
                                    ),
                                    None,
                                );
                            }
                        }
                    }
                });

                if let Ok(Some(id)) = selection_recv.try_next() {
                    if selected_id.is_some_and(|v| v == id) {
                        selected_id = None;
                    } else {
                        selected_id = Some(id);
                    }
                }

                let mouse_pos = camera.cursor_to_pos(input_state.main_pos, point3(0.0, 0.0, 1.0));
                let mouse_pos = mouse_pos + camera.camera_state().pos.to_vec().truncate();

                let camera_state = camera.camera_state();
                if camera_state.is_at_max_height() {
                    if let Some(id) = selected_id {
                        if let Some(faces_index) = init_data
                            .resource_man
                            .resources
                            .get(&id)
                            .and_then(|v| v.faces_index)
                        {
                            let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

                            let glow = (time.as_secs_f64() * 3.0).sin() / 10.0;

                            let instance = InstanceData::new()
                                .faces_index(faces_index)
                                .position_offset([mouse_pos.x as Num, mouse_pos.y as Num, 0.1])
                                .color_offset(Color::TRANSPARENT.with_alpha(glow as Num).into());
                            extra_instances.push(instance);
                        }
                    }
                }

                let render_info: MapRenderInfo = block_on(ask(
                    &sys,
                    &game,
                    GameMsg::RenderInfoRequest {
                        context: RenderContext {
                            init_data: init_data.clone(),
                        },
                    },
                ));

                renderer.render(
                    render_info,
                    camera.camera_state(),
                    none,
                    extra_instances,
                    extra_vertices,
                    &mut gui,
                );
            }
        });
    }
}
