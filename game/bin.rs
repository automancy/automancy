#![feature(result_option_inspect)]

use automancy::{
    data::map::Map,
    game::{
        game::Game,
        ticking::TICK_INTERVAL,
    },
    render::{
        camera::Camera,
        data::Model,
        gpu::{self, frag_shader, vert_shader},
        renderer::Renderer,
    }
};

use json::JsonValue;

use automancy::registry::init::InitData;
use automancy::util::{
    id::Id,
    resource::ResourceManager,
};
use walkdir::WalkDir;

use std::{
    ffi::OsStr,
    fs::{File, read_to_string},
    path::Path,
    sync::Arc,
};
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_4, FRAC_PI_8, PI};
use std::sync::Mutex;
use cgmath::{point3, SquareMatrix, vec3};
use egui::panel::{Side, TopBottomSide};
use egui::{Align, Align2, Area, Color32, Frame, Layout, PaintCallback, Rect, Rounding, ScrollArea, Sense, Shape, Stroke, Style, TopBottomPanel, Vec2, vec2, Visuals, Window};
use egui::epaint::Shadow;
use egui::style::Margin;
use egui_winit_vulkano::{CallbackFn, Gui};
use futures::executor::block_on;

use riker::actors::{ActorRefFactory, SystemBuilder, Timer};

use vulkano::buffer::BufferUsage;

use vulkano::command_buffer::{DrawIndexedIndirectCommand, PrimaryCommandBufferAbstract};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Features, QueueCreateInfo};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::format::Format;
use vulkano::image::{AttachmentImage, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::instance::debug::ValidationFeatureEnable;
use vulkano::memory::allocator::{FastMemoryAllocator, StandardMemoryAllocator};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::render_pass::Subpass;
use vulkano::swapchain::{Swapchain, SwapchainCreateInfo};
use vulkano::sync::GpuFuture;
use vulkano::VulkanLibrary;
use vulkano_win::create_surface_from_winit;

use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};
use automancy::data::tile::{Tile, TileCoord};
use automancy::input::{handler::InputHandler, primitive::convert_input};
use automancy::game::game::GameMsg;
use automancy::math::cg::{eye, matrix, Matrix4, Num, perspective, projection, Vector3, view};
use automancy::render::data::{InstanceData, UniformBufferObject, Vertex};
use automancy::render::gpu::{Gpu, gui_frag_shader, gui_vert_shader, indirect_buffer};
use automancy::render::gui;
use automancy::util::colors::Color;

pub const ASSET_LOGO: &str = "assets/logo.png";

pub const RESOURCE: &str = "resources";

fn load_resources() -> (ResourceManager, Vec<Option<(Id, Option<Model>)>>) {
    let mut resource_man = ResourceManager::default();

    // TODO: just use serde?

    let resources = WalkDir::new(RESOURCE)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let path = entry.into_path();
            let working_dir = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();

            let extension = path.extension().and_then(OsStr::to_str);

            if let Some("json") = extension {
                log::info!("loading resource at {:?}", path);

                let json = json::parse(read_to_string(&path).unwrap().as_str());

                match json {
                    Ok(JsonValue::Object(json)) => return Some((json, working_dir)),
                    Err(err) => {
                        log::warn!("error while reading resource: {:?}. error: {}", path, err);
                    }
                    _ => (),
                }
            }

            None
        })
        .map(|(json, working_dir)| {
            resource_man.load_resource(json, &working_dir)
        })
        .collect::<Vec<_>>();

    (resource_man, resources)
}

fn init() -> Arc<InitData> {
    let (resource_man, resources) = load_resources();

    let init_data = InitData::new(resource_man, resources);

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
            .with_inner_size(PhysicalSize::new(800, 800))
            .build(&event_loop)
            .expect("could not build window")
    );

    let surface = create_surface_from_winit(window.clone(), instance.clone()).expect("could not create surface");

    // --- physical device ---
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        khr_depth_stencil_resolve: true,
        ..DeviceExtensions::empty()
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
            color: {
                load: Clear,
                store: Store,
                format: swapchain.image_format(),
                samples: 1,
            },
            depth: {
                load: Clear,
                store: DontCare,
                format: Format::D24_UNORM_S8_UINT,
                samples: 1,
            },
            depth_gui: {
                load: Clear,
                store: DontCare,
                format: Format::D24_UNORM_S8_UINT,
                samples: 1,
            }
        },
        passes: [
            { color: [color], depth_stencil: { depth }, input: [] },
            { color: [color], depth_stencil: { depth_gui }, input: [] }
        ]
    ).unwrap();
    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
    let gui_subpass = Subpass::from(render_pass.clone(), 1).unwrap();

    // --- buffers ---
    let allocator = StandardMemoryAllocator::new_default(device.clone());

    let mut command_buffer_builder = gpu::command_buffer_builder(device.clone(), queue.clone());

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
            .all_faces
            .iter()
            .flatten()
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

    let depth_buffer = AttachmentImage::with_usage(
        &allocator,
        gpu::window_size_u32(&window),
        Format::D24_UNORM_S8_UINT,
        ImageUsage {
            depth_stencil_attachment: true,
            ..Default::default()
        },
    ).unwrap();

    let depth_buffer_gui = AttachmentImage::with_usage(
        &allocator,
        gpu::window_size_u32(&window),
        Format::D24_UNORM_S8_UINT,
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
            .render_pass(gui_subpass.clone());

        pipeline.build(device.clone()).unwrap()
    };

    // --- framebuffers ---
    let framebuffers = gpu::framebuffers(&images, render_pass.clone(), depth_buffer.clone(), depth_buffer_gui.clone());

    // --- gpu ---
    let gpu = Arc::new(Gpu {
        device,
        queue,
        surface,
        render_pass,

        window,

        vertex_buffer,
        index_buffer,
        uniform_buffer, // TODO move uniform and depth buffer
    });

    // --- load map ---
    //let map = Map::load("test".to_owned());
    let map = Map::new_empty("test".to_owned());

    // --- init actors ---
    let sys = SystemBuilder::new().name("automancy").create().unwrap();

    let game = sys.actor_of_args::<Game, Map>("game", map).unwrap();

    let mut input_handler = InputHandler::new();
    let mut camera = Camera::new(gpu::window_size(&gpu.window));
    let mut renderer = Renderer::new(
        game.clone(),
        init_data.clone(),
        gpu.clone(),
        game_pipeline,
        depth_buffer,
        depth_buffer_gui,
        swapchain,
        framebuffers
    );

    // --- game ---
    let tick = GameMsg::Tick {};

    sys.schedule(TICK_INTERVAL, TICK_INTERVAL, game.clone(), None, tick);

    let mut gui = Gui::new_with_subpass(
        &event_loop,
        gpu.surface.clone(),
        None,
        gpu.queue.clone(),
        gui_subpass,
    );

    // --- event-loop ---
    {
        let mut closed = false;

        event_loop.run(move |event, _, control_flow| {
            if closed {
                return;
            }

            let mut window_event = None;
            let mut device_event = None;

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
                    gpu.window.request_redraw();
                }

                _ => {},
            };

            let input_event = convert_input(window_event, device_event);
            if input_event.device.is_some() || input_event.window.is_some() {
                let input_state = input_handler.update(input_event);
                camera.input_state(input_state);
            }

            if event == Event::RedrawRequested(gpu.window.id()) {
                let gpu = gpu.clone();

                camera.update_pos();
                camera.update_pointing_at();

                gui.begin_frame();

                TopBottomPanel::bottom("tiles")
                    .show_separator_line(false)
                    .resizable(false)
                    .frame(
                        Frame::none()
                            .fill(Color::WHITE.with_alpha(0.5).into())
                            .shadow(Shadow {
                                extrusion: 12.0,
                                color: Color::GRAY.with_alpha(0.5).into(),
                            })
                            .outer_margin(Margin::same(10.0))
                            .rounding(Rounding::same(5.0))
                    )
                    .show(&gui.context(), |ui| {
                        let style = ui.style_mut();

                        style.spacing.interact_size.y = 70.0;
                        style.spacing.scroll_bar_width = 0.0;
                        style.spacing.scroll_bar_outer_margin = 0.0;

                        ScrollArea::horizontal()
                            .always_show_scroll(true)
                            .show_viewport(ui, |ui, _viewport| {
                                ui.horizontal(|ui| {
                                    gui::render_tile_selection(ui, init_data.clone(), gpu.clone(), gui_pipeline.clone());
                                });
                            });
                    });

                block_on(renderer.render(&sys, camera.get_camera_state(), subpass.clone(), &mut gui));
            }
        });
    }
}
