#![feature(result_option_inspect)]
#![feature(is_some_and)]

use std::{ffi::OsStr, fs::{File, read_to_string}, fs, sync::Arc};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use cgmath::{EuclideanSpace, point3};
use egui::{Align, Align2, Color32, FontData, FontDefinitions, FontId, Frame, Rounding, ScrollArea, Stroke, Style, TextStyle, TopBottomPanel, vec2, Visuals, Window};
use egui::epaint::Shadow;
use egui::FontFamily::{Monospace, Proportional};
use egui::style::{Margin, Widgets, WidgetVisuals};
use egui_winit_vulkano::Gui;
use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures::executor::block_on;
use hexagon_tiles::hex::Hex;
use riker::actor::ActorRef;
use riker::actors::{ActorRefFactory, SystemBuilder, Tell, Timer};
use riker_patterns::ask::ask;
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBufferAbstract};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Features, QueueCreateInfo};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::format::Format;
use vulkano::image::{AttachmentImage, ImageUsage};
use vulkano::image::SampleCount::Sample4;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::Subpass;
use vulkano::swapchain::{Swapchain, SwapchainCreateInfo};
use vulkano::sync::GpuFuture;
use vulkano::VulkanLibrary;
use vulkano_win::create_surface_from_winit;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};

use automancy::{
    game::{
        game::Game,
        ticking::TICK_INTERVAL,
    },
    game::map::Map,
    render::{
        camera::Camera,
        gpu::{self, frag_shader, vert_shader},
        renderer::Renderer,
    }
};
use automancy::data::data::Data;
use automancy::data::id::Id;
use automancy::game::game::GameMsg;
use automancy::game::input::convert_input;
use automancy::game::input::InputState;
use automancy::game::map::{MapRenderInfo, RenderContext};
use automancy::game::tile::{TileCoord, TileMsg, TileUnit};
use automancy::render::data::{InstanceData, Vertex};
use automancy::render::gpu::{Gpu, gui_frag_shader, gui_vert_shader};
use automancy::render::gui;
use automancy::util::cg::Num;
use automancy::util::colors::Color;
use automancy::util::init::InitData;
use automancy::util::resource::{ResourceManager, ResourceRaw, ResourceType};

pub const ASSET_LOGO: &str = "assets/logo.png";

pub const RESOURCE: &str = "resources";

fn load_resources() -> ResourceManager {
    let mut resource_man = ResourceManager::new();

    // TODO: just use serde?

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

    resource_man
}

fn init() -> Arc<InitData> {
    let resource_man = load_resources();

    let init_data = InitData::new(resource_man);

    Arc::new(init_data)
}

fn main() {
    env_logger::init();

    // --- resources & data ---
    let init_data = init();

    // --- instance ---
    let library = VulkanLibrary::new().expect("no local Vulkan library");
    let required_extensions = vulkano_win::required_extensions(&library);

    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            enabled_extensions: required_extensions,
            enumerate_portability: true,
            ..Default::default()
        }
    ).expect("failed to create instance");

    // --- surface ---
    let (bytes, width, height) = {
        let decoder = png::Decoder::new(File::open(ASSET_LOGO).unwrap());

        let mut reader = decoder.read_info().unwrap();

        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();

        (buf[..info.buffer_size()].to_vec(), info.width, info.height)
    };

    let icon = Icon::from_rgba(bytes, width, height).unwrap();

    let event_loop = EventLoop::new();

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("automancy")
            .with_window_icon(Some(icon))
            .build(&event_loop)
            .expect("could not build window")
    );

    let surface = create_surface_from_winit(window.clone(), instance.clone()).expect("could not create surface");

    // --- physical device ---
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        khr_dedicated_allocation: true,
        khr_get_memory_requirements2: true,
        ..DeviceExtensions::default()
    };

    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()
        .expect("could not enumerate devices")
        .filter(|p| p.supported_extensions().contains(&device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.graphics && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|q| (p, q as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            _ => 4,
        })
        .expect("no devices available");

    log::info!(
        "Using device: {} (type: {:?})",
        physical_device.properties().device_name,
        physical_device.properties().device_type
    );

    // --- logical device ---
    let (device, mut queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_extensions: device_extensions,
            enabled_features: Features {
                multi_draw_indirect: true,
                fill_mode_non_solid: true,
                ..Default::default()
            },
            ..Default::default()
        },
    ).expect("failed to create device");

    // --- queue ---
    let queue = queues.next().unwrap();

    // --- swapchain ---
    let (swapchain, images) = {
        let surface_capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        let image_format = Some(
            physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        log::debug!("image_format: {:?}", image_format);

        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count,

                image_format,
                image_extent: gpu::window_size_u32(&window),

                image_usage: ImageUsage {
                    color_attachment: true,
                    transfer_dst: true,
                    ..Default::default()
                },
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .iter()
                    .next()
                    .unwrap(),

                ..Default::default()
            },
        ).unwrap()
    };

    // --- render pass ---
    let render_pass = vulkano::ordered_passes_renderpass!(
        device.clone(),
        attachments: {
            color_resolve: {
                load: DontCare,
                store: Store,
                format: swapchain.image_format(),
                samples: 1,
            },
            color: {
                load: Clear,
                store: Store,
                format: swapchain.image_format(),
                samples: 4,
            },
            depth: {
                load: Clear,
                store: DontCare,
                format: Format::D32_SFLOAT,
                samples: 4,
            },
            depth_gui: {
                load: Clear,
                store: DontCare,
                format: Format::D32_SFLOAT,
                samples: 4,
            }
        },
        passes: [
            { color: [color], depth_stencil: { depth     }, input: [], resolve: [color_resolve] },
            { color: [color], depth_stencil: { depth_gui }, input: [], resolve: [color_resolve] }
        ]
    ).unwrap();
    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
    let gui_subpass = Subpass::from(render_pass.clone(), 1).unwrap();

    // --- buffers ---
    let allocator = StandardMemoryAllocator::new_default(device.clone());

    let command_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo {
            ..Default::default()
        }
    );

    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        &command_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    ).unwrap();

    let vertex_buffer = gpu::immutable_buffer(
        &allocator,
        init_data.combined_vertices.clone(),
        BufferUsage {
            vertex_buffer: true,
            ..Default::default()
        },
        &mut command_buffer_builder,
    );
    let index_buffer = gpu::immutable_buffer(
        &allocator,
        init_data
            .all_raw_faces
            .iter()
            .flatten()
            .flat_map(|v| v.indices.clone())
            .collect::<Vec<_>>(),
        BufferUsage {
            index_buffer: true,
            ..Default::default()
        },
        &mut command_buffer_builder,
    );
    let uniform_buffer = gpu::uniform_buffer(&allocator);
    let gui_uniform_buffer = gpu::uniform_buffer(&allocator);

    let color_image = AttachmentImage::multisampled_with_usage(
        &allocator,
        gpu::window_size_u32(&window),
        Sample4,
        swapchain.image_format(),
        ImageUsage {
            color_attachment: true,
            ..Default::default()
        },
    ).unwrap();

    let depth_buffer = AttachmentImage::multisampled_with_usage(
        &allocator,
        gpu::window_size_u32(&window),
        Sample4,
        Format::D32_SFLOAT,
        ImageUsage {
            depth_stencil_attachment: true,
            ..Default::default()
        },
    ).unwrap();

    let depth_buffer_gui = AttachmentImage::multisampled_with_usage(
        &allocator,
        gpu::window_size_u32(&window),
        Sample4,
        Format::D32_SFLOAT,
        ImageUsage {
            depth_stencil_attachment: true,
            ..Default::default()
        },
    ).unwrap();

    let command_buffer = command_buffer_builder.build().unwrap();
    block_on(
        command_buffer
            .execute(queue.clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
    ).unwrap();

    drop(allocator);

    // --- shaders ---
    let vs = vert_shader::load(device.clone()).unwrap();
    let fs = frag_shader::load(device.clone()).unwrap();

    let vs_gui = gui_vert_shader::load(device.clone()).unwrap();
    let fs_gui = gui_frag_shader::load(device.clone()).unwrap();

    // --- pipeline ---
    let game_pipeline = {
        let pipeline = GraphicsPipeline::start()
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>().instance::<InstanceData>())
            .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleList))
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .rasterization_state(RasterizationState::new())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .multisample_state(MultisampleState {
                rasterization_samples: Sample4,
                ..Default::default()
            })
            .render_pass(subpass.clone());

        pipeline.build(device.clone()).unwrap()
    };

    let gui_pipeline = {
        let pipeline = GraphicsPipeline::start()
            .vertex_shader(vs_gui.entry_point("main").unwrap(), ())
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleList))
            .fragment_shader(fs_gui.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_dynamic(1))
            .rasterization_state(RasterizationState::new())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .multisample_state(MultisampleState {
                rasterization_samples: Sample4,
                ..Default::default()
            })
            .render_pass(gui_subpass.clone());

        pipeline.build(device.clone()).unwrap()
    };

    // --- framebuffers ---
    let framebuffers = gpu::framebuffers(&images, render_pass.clone(), color_image.clone(), depth_buffer.clone(), depth_buffer_gui.clone());

    // --- gpu ---
    let gpu = Arc::new(Gpu {
        device,
        queue,
        surface,
        render_pass,

        window,

        vertex_buffer,
        index_buffer,
    });

    // --- load map ---
    //let map = Map::load("test".to_owned());
    let map = Map::new_empty("test".to_owned());

    // --- init actors ---
    let sys = SystemBuilder::new().name("automancy").create().unwrap();

    let game = sys.actor_of_args::<Game, Arc<Map>>("bin", Arc::new(map)).unwrap();

    let mut camera = Camera::new(gpu::window_size(&gpu.window));
    let mut renderer = Renderer::new(
        init_data.clone(),

        gpu.clone(),
        game_pipeline,
        swapchain,
        framebuffers,

        command_allocator,
        StandardDescriptorSetAllocator::new(gpu.device.clone()),

        color_image,
        depth_buffer,
        depth_buffer_gui,

        uniform_buffer,
        gui_uniform_buffer,
    );

    let none = init_data.resource_man.none;

    // --- bin ---
    let tick = GameMsg::Tick {
        init_data: init_data.clone(),
    };

    let _id = sys.schedule(TICK_INTERVAL, TICK_INTERVAL, game.clone(), None, tick);

    let mut gui = Gui::new_with_subpass(
        &event_loop,
        gpu.surface.clone(),
        Some(renderer.swapchain.image_format()),
        gpu.queue.clone(),
        gui_subpass.clone(),
    );

    log::info!("loading font...");
    {
        let mut fonts = FontDefinitions::default();
        let iosevka = "iosevka".to_owned();

        fonts.font_data.insert(iosevka.clone(), FontData::from_static(include_bytes!("./fonts/iosevka-extended.ttf")));

        fonts.families.get_mut(&Proportional).unwrap()
            .insert(0, iosevka.clone());
        fonts.families.get_mut(&Monospace).unwrap()
            .insert(0, iosevka.clone());

        gui.context().set_fonts(fonts);
    }

    {
        gui.context().set_style(
            Style {
                override_text_style: None,
                override_font_id: None,
                text_styles: [
                    (TextStyle::Small, FontId::new(9.0, Proportional)),
                    (TextStyle::Body, FontId::new(13.0, Proportional)),
                    (TextStyle::Button, FontId::new(13.0, Proportional)),
                    (TextStyle::Heading, FontId::new(19.0, Proportional)),
                    (TextStyle::Monospace, FontId::new(13.0, Monospace)),
                ].into(),
                wrap: None,
                visuals: Visuals {
                    widgets: Widgets {
                        noninteractive: WidgetVisuals {
                            bg_fill: Color32::from_gray(190),
                            bg_stroke: Stroke::new(1.0, Color32::from_gray(190)), // separators, indentation lines
                            fg_stroke: Stroke::new(1.0, Color32::from_gray(80)),  // normal text color
                            rounding: Rounding::same(2.0),
                            expansion: 0.0,
                        },
                        inactive: WidgetVisuals {
                            bg_fill: Color32::from_gray(180), // button background
                            bg_stroke: Default::default(),
                            fg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // button text
                            rounding: Rounding::same(2.0),
                            expansion: 0.0,
                        },
                        hovered: WidgetVisuals {
                            bg_fill: Color32::from_gray(170),
                            bg_stroke: Stroke::new(1.0, Color32::from_gray(105)), // e.g. hover over window edge or button
                            fg_stroke: Stroke::new(1.5, Color32::BLACK),
                            rounding: Rounding::same(3.0),
                            expansion: 1.0,
                        },
                        active: WidgetVisuals {
                            bg_fill: Color32::from_gray(160),
                            bg_stroke: Stroke::new(1.0, Color32::BLACK),
                            fg_stroke: Stroke::new(2.0, Color32::BLACK),
                            rounding: Rounding::same(2.0),
                            expansion: 1.0,
                        },
                        open: WidgetVisuals {
                            bg_fill: Color32::from_gray(170),
                            bg_stroke: Stroke::new(1.0, Color32::from_gray(160)),
                            fg_stroke: Stroke::new(1.0, Color32::BLACK),
                            rounding: Rounding::same(2.0),
                            expansion: 0.0,
                        },
                    },
                    ..Visuals::light()
                },
                ..Default::default()
            }
        );
    }

    let frame = Frame::none()
        .fill(Color::WHITE.with_alpha(0.7).into())
        .shadow(Shadow {
            extrusion: 8.0,
            color: Color::DARK_GRAY.with_alpha(0.5).into(),
        })
        .rounding(Rounding::same(5.0));

    // --- event-loop ---
    {
        let mut closed = false;

        let mut input_state = InputState::default();

        let mut selected_id: Option<Id> = None;
        let mut pointing_at = TileCoord::ZERO;

        let mut config_open = None;

        let mut script_filter = String::new();

        let fuse = Fuse::default();

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

                    *control_flow = ControlFlow::Exit;

                    closed = true;

                    return;
                },

                Event::WindowEvent { event, .. } => {
                    if !gui.update(&event) {
                        window_event = Some(event);
                    }

                    match event {
                        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                            renderer.recreate_swapchain = true;
                            camera.window_size = gpu::window_size(&gpu.window);
                        }
                        _ => {}
                    }
                },

                Event::DeviceEvent { event, .. } => {
                    device_event = Some(event);
                },

                Event::MainEventsCleared => {
                    camera.update_pos();
                    camera.update_pointing_at(input_state.main_pos);
                    gpu.window.request_redraw();
                }

                _ => {},
            };

            if window_event.is_some() || device_event.is_some() {
                input_state.reset();

                input_state.update(convert_input(window_event, device_event));

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
                            None
                        );
                    }
                }

                if input_state.alternate_pressed {
                    if config_open == Some(pointing_at)  {
                        config_open = None;
                        script_filter.clear();
                    } else {
                        config_open = Some(pointing_at);
                    }
                }
            }

            if event == Event::RedrawRequested(gpu.window.id()) {
                let gpu = gpu.clone();

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
                                        gui::render_tile_selection(ui, init_data.clone(), selection_send, gpu.clone(), gui_pipeline.clone());
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

                            let result: Option<(Id, ActorRef<TileMsg>)> = block_on(ask(&sys, &game, GameMsg::GetTile(coord)));

                            if let Some((id, tile)) = result {
                                ui.label(init_data.resource_man.tile_name(&id));
                                let data: Data = block_on(ask(&sys, &tile, TileMsg::GetData));

                                for (id, amount) in data.0.iter() {
                                    ui.label(format!("{} - {}", init_data.resource_man.item_name(id), amount));
                                }
                            }
                        });

                    if let Some(config_open) = config_open {
                        let result: Option<(Id, ActorRef<TileMsg>)> = block_on(ask(&sys, &game, GameMsg::GetTile(config_open)));

                        if let Some((id, tile)) = result {
                            let current_script: Option<Id> = block_on(ask(&sys, &tile, TileMsg::GetScript));
                            let mut new_script = current_script.clone();

                            let current_target_coord: Option<Hex<TileUnit>> = block_on(ask(&sys, &tile, TileMsg::GetTarget));
                            let mut new_target_coord = current_target_coord;

                            // tile_config
                            if let ResourceType::Machine(_) = init_data.resource_man.resources[&id].resource_type {
                                if let Some(scripts) = init_data.resource_man.resources[&id].scripts.clone() {
                                    Window::new("Config")
                                        .resizable(false)
                                        .auto_sized()
                                        .constrain(true)
                                        .frame(frame.inner_margin(Margin::same(10.0)))
                                        .show(&gui.context(), |ui| {
                                            ui.set_max_width(300.0);

                                            let script_text = if let Some(ref script) = new_script {
                                                init_data.resource_man.item_name(script) //TODO move this to resource_man
                                            } else {
                                                "<none>".to_owned()
                                            };

                                            ui.label(format!("Script: {}", script_text));

                                            ui.text_edit_singleline(&mut script_filter);

                                            ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                                                ui.set_width(ui.available_width());

                                                let scripts = if !script_filter.is_empty() {
                                                    let mut filtered = scripts
                                                        .into_iter()
                                                        .flat_map(|id| {
                                                            let result = fuse.search_text_in_string(&script_filter, init_data.resource_man.item_name(&id).as_str());

                                                            Some(id).zip(result.map(|v| v.score))
                                                        })
                                                        .collect::<Vec<_>>();

                                                    filtered.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

                                                    filtered.into_iter().map(|v| v.0).collect::<Vec<_>>()
                                                } else {
                                                    scripts
                                                };

                                                scripts.iter().for_each(|script| {
                                                    ui.radio_value(
                                                        &mut new_script,
                                                        Some(*script),
                                                        init_data.resource_man.item_name(&script)
                                                    );
                                                })
                                            });

                                            ui.separator();

                                            ui.label("Target:");
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(15.0);
                                                    gui::add_direction(ui, &mut new_target_coord, 5);
                                                    gui::add_direction(ui, &mut new_target_coord, 0);
                                                });

                                                ui.horizontal(|ui| {
                                                    gui::add_direction(ui, &mut new_target_coord, 4);
                                                    ui.selectable_value(&mut new_target_coord, None, "❌");
                                                    gui::add_direction(ui, &mut new_target_coord, 1);
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.add_space(15.0);
                                                    gui::add_direction(ui, &mut new_target_coord, 3);
                                                    gui::add_direction(ui, &mut new_target_coord, 2);
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
                                        TileMsg::SetTarget(new_target_coord)
                                    ),
                                    None
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
                        if let Some(faces_index) = init_data.resource_man.resources.get(&id).and_then(|v| v.faces_index) {
                            let time = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap();

                            let glow = (time.as_secs_f64() * 3.0).sin() / 10.0;

                            let instance = InstanceData::new()
                                .faces_index(faces_index)
                                .position_offset([mouse_pos.x as Num, mouse_pos.y as Num, 0.1])
                                .color_offset(Color::TRANSPARENT.with_alpha(glow as Num).into());
                            extra_instances.push(instance);
                        }
                    }
                }

                let render_info: MapRenderInfo = block_on(
                    ask(&sys, &game, GameMsg::RenderInfoRequest {
                        context: RenderContext {
                            init_data: init_data.clone(),
                        }
                    })
                );

                renderer.render(render_info, camera.camera_state(), none, subpass.clone(), gui_subpass.clone(), extra_instances, extra_vertices, &mut gui, gui_pipeline.clone());
            }
        });
    }
}
