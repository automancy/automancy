#![feature(split_array)]
#![feature(array_chunks)]
#![feature(slice_group_by)]
#![feature(duration_consts_float)]

#[macro_use]
mod util;
#[macro_use]
mod game;
mod actor;
mod math;
mod registry;

use crate::game::{
    data::pos::Pos,
    game::{Game, LoadChunkRange, WorldRenderContextRequest},
    player::input_handler::{convert_input, CursorStateRequest, InputHandler},
    render::{
        camera::{Camera, CameraPosRequest, CameraRayRequest, MAX_CAMERA_Z},
        data::InstanceData,
        gpu::{self, dbg_frag_shader, dbg_vert_shader, frag_shader, vert_shader},
        renderer::{DrawInfo, Redraw, Renderer},
    },
    ticking::{TICK_INTERVAL, TPS},
};
use actix::{clock::interval, Actor, Recipient, System};

use cgmath::point2;
use game::{
    game::Tick,
    render::{
        data::{UniformBufferObject, Vertex},
        gpu::window_size_u32,
    },
    ticking::MAX_ALLOWED_TICK_INTERVAL,
};
use registry::init::InitData;
use util::resource::{load_resource, Resource};
use walkdir::WalkDir;

use std::{fs::File, sync::Arc, thread, time::Instant};

use tokio::{
    fs::read_to_string,
    runtime::Runtime,
    sync::{oneshot, OnceCell},
};
use tokio::{sync::watch, time::Interval};

use vulkano::{
    buffer::{
        cpu_pool::CpuBufferPoolChunk, BufferAccess, BufferSlice, BufferUsage, CpuAccessibleBuffer,
        CpuBufferPool, ImmutableBuffer,
    },
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo},
    format::{ClearValue, Format},
    image::{AttachmentImage, ImageAccess, ImageUsage},
    instance::{Instance, InstanceCreateInfo},
    memory::pool::StdMemoryPool,
    pipeline::{
        graphics::{
            input_assembly::PrimitiveTopology,
            rasterization::{PolygonMode, RasterizationState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint, StateMode,
    },
    render_pass::{Framebuffer, RenderPass},
    swapchain::{
        acquire_next_image, AcquireError, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;

use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, Window, WindowBuilder},
};

pub const ASSET_LOGO: &str = "assets/logo.png";

// TODO: use metadata file + directory scan
pub const RESOURCE: &str = "resources";

pub type Re = OnceCell<Resource>;

async fn load_resources() -> Vec<Resource> {
    let mut resources = vec![];

    for entry in WalkDir::new(RESOURCE) {
        if let Ok(entry) = entry {
            let path = entry.into_path();
            let extension = path.extension().and_then(|v| v.to_str());

            if let Some("json") = extension {
                log::info!("loading resource at {:?}", path);

                let json = json::parse(read_to_string(&path).await.unwrap().as_str());

                if let Err(err) = json {
                    log::warn!("error while reading resource: {:?}. error: {}", path, err);

                    continue;
                }
                let json = json.unwrap();

                resources.push(load_resource(json, &path).await);
            }
        }
    }

    resources
}

fn render(
    draw_info: &DrawInfo,
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Arc<Surface<Window>>,
    render_pass: Arc<RenderPass>,
    swapchain: &mut Arc<Swapchain<Window>>,
    recreate_swapchain: &mut bool,
    framebuffers: &mut Vec<Arc<Framebuffer>>,
    previous_frame_end: &mut Option<Box<dyn GpuFuture + Send + Sync>>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_buffer: Arc<ImmutableBuffer<[Vertex]>>,
    index_buffer: Arc<ImmutableBuffer<[u32]>>,
    debug_pipeline: Arc<GraphicsPipeline>,
    debug_vertex_buffer: Arc<BufferSlice<[Vertex], CpuBufferPoolChunk<Vertex, Arc<StdMemoryPool>>>>,
    depth_buffer: &mut Arc<AttachmentImage>,
    uniform_buffers: &mut Vec<Arc<CpuAccessibleBuffer<UniformBufferObject>>>,
    instance_buffer: &[InstanceData],
    init_data: &InitData,
) {
    let size = window_size_u32(&surface);

    if size[0] == 0 || size[1] == 0 {
        return;
    }

    if ImageAccess::dimensions(depth_buffer).width_height() != size {
        *recreate_swapchain = true;

        *depth_buffer = AttachmentImage::with_usage(
            device.clone(),
            size,
            Format::D24_UNORM_S8_UINT,
            ImageUsage::depth_stencil_attachment(),
        )
        .unwrap();
    }

    previous_frame_end.as_mut().unwrap().cleanup_finished();

    if *recreate_swapchain {
        let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
            image_extent: size.into(),
            ..swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("failed to recreate swapchain: {:?}", e),
        };

        *swapchain = new_swapchain;

        *framebuffers = gpu::framebuffers(&new_images, render_pass.clone(), depth_buffer.clone());
        *recreate_swapchain = false;
    }

    let (image_num, suboptimal, acquire_future) = match acquire_next_image(swapchain.clone(), None)
    {
        Ok(r) => r,
        Err(AcquireError::OutOfDate) => {
            *recreate_swapchain = true;
            return;
        }
        Err(e) => panic!("failed to acquire next image: {:?}", e),
    };

    if suboptimal {
        *recreate_swapchain = true;
    }

    let indirect_instance = if !instance_buffer.is_empty() {
        let (instance_buffer, commands) =
            gpu::indirect_buffer(queue.clone(), init_data, instance_buffer);

        if !commands.is_empty() {
            let indirect_buffer = gpu::immutable_buffer(
                commands.into_iter(),
                BufferUsage::indirect_buffer(),
                queue.clone(),
            );
            Some((indirect_buffer, instance_buffer))
        } else {
            None
        }
    } else {
        None
    };

    let ubo = UniformBufferObject {
        matrix: draw_info.view.into(),
    };

    *uniform_buffers[image_num].write().unwrap() = ubo;

    let pipeline_layout = pipeline.layout();
    let ubo_layout = pipeline_layout.set_layouts()[0].clone();
    let ubo_set = PersistentDescriptorSet::new(
        ubo_layout.clone(),
        [WriteDescriptorSet::buffer(
            0,
            uniform_buffers[image_num].clone(),
        )],
    )
    .unwrap();

    /*
    let debug_pipeline_layout = debug_pipeline.layout();
    let debug_ubo_layout = debug_pipeline_layout.set_layouts()[0].clone();
    let debug_ubo_set = PersistentDescriptorSet::new(
        debug_ubo_layout.clone(),
        [WriteDescriptorSet::buffer(
            0,
            uniform_buffers[image_num].clone(),
        )],
    )
    .unwrap();
     */

    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    builder
        .begin_render_pass(
            framebuffers[image_num].clone(),
            SubpassContents::Inline,
            [
                ClearValue::Float([0.0, 0.0, 0.0, 1.0]),
                ClearValue::DepthStencil((1.0, 0)),
            ],
        )
        .unwrap()
        .set_viewport(0, [gpu::viewport(&surface)]);

    if let Some((indirect_buffer, instance_buffer)) = indirect_instance {
        builder
            .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer.clone()))
            .bind_index_buffer(index_buffer.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline_layout.clone(),
                0,
                ubo_set,
            )
            .bind_pipeline_graphics(pipeline.clone())
            .draw_indexed_indirect(indirect_buffer)
            .unwrap();
    }

    let debug_vertex_buffer_size = debug_vertex_buffer.len() as u32;
    if debug_vertex_buffer_size != 0 {
        builder
            .bind_vertex_buffers(0, debug_vertex_buffer)
            .bind_pipeline_graphics(debug_pipeline.clone())
            /*
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                debug_pipeline_layout.clone(),
                0,
                debug_ubo_set,
            )
            */
            .draw(debug_vertex_buffer_size, 1, 0, 0)
            .unwrap();
    }

    builder.end_render_pass().unwrap();

    let command_buffer = builder.build().unwrap();

    let future = previous_frame_end
        .take()
        .unwrap()
        .join(acquire_future)
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
        .then_signal_fence_and_flush();

    match future {
        Ok(future) => {
            future.wait(None).unwrap();
            *previous_frame_end = Some(future.boxed_send_sync());
        }
        Err(FlushError::OutOfDate) => {
            *recreate_swapchain = true;
            *previous_frame_end = Some(sync::now(device.clone()).boxed_send_sync());
        }
        Err(e) => {
            log::error!("failed to flush future: {:?}", e);
            *previous_frame_end = Some(sync::now(device.clone()).boxed_send_sync());
        }
    }
}

pub async fn game_tick(
    game: &Recipient<Tick>,
    interval: &mut Interval,
    counter: &mut u64,
    start: &mut Option<Instant>,
) -> Option<()> {
    let game_tick_start = Instant::now();

    let _ = game.send(Tick()).await.unwrap();

    let game_tick_end = Instant::now();

    let tick_time = game_tick_end.duration_since(game_tick_start);

    if tick_time > MAX_ALLOWED_TICK_INTERVAL {
        log::warn!(
            "tick took longer than allowed maximum! tick_time: {:?}, maximum: {:?}",
            tick_time,
            MAX_ALLOWED_TICK_INTERVAL
        );
    }

    *counter += 1;
    if *counter >= TPS {
        // todo: what to do when the TPS is low?
        {
            if let Some(start) = start {
                let elapsed = start.elapsed();
                let tps = (*counter * 1_000_000_000) as f64 / elapsed.as_nanos() as f64;
                let tps = (tps * 100.0).round() / 100.0;

                log::debug!("TPS: {:.1}, elapsed time: {:?}", tps, elapsed);
            }
            start.replace(Instant::now());
        }

        *counter -= TPS;
    }

    interval.tick().await;

    Some(())
}

async fn init() -> Arc<InitData> {
    let resources = load_resources().await;

    let init_data = InitData::new(resources);

    Arc::new(init_data)
}

fn main() {
    env_logger::init();

    // --- resources & data ---
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    let init_data = handle.block_on(init());

    // --- instance ---
    let required_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(InstanceCreateInfo {
        enabled_extensions: required_extensions,
        ..Default::default()
    })
    .expect("failed to create instance");

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
    let surface = WindowBuilder::new()
        .with_title("automancy")
        .with_window_icon(Some(icon))
        .with_inner_size(PhysicalSize::new(800, 800))
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        khr_storage_buffer_storage_class: true,
        khr_depth_stencil_resolve: true,
        ..DeviceExtensions::none()
    };

    // --- physical device ---
    let (physical, queue_family) =
        gpu::select_physical_device(&instance, surface.clone(), &device_extensions);
    log::info!(
        "Using device: {} (type: {:?})",
        physical.properties().device_name,
        physical.properties().device_type
    );

    // --- logical device ---
    let (device, mut queues) = Device::new(
        physical,
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
            enabled_extensions: physical.required_extensions().union(&device_extensions),
            enabled_features: Features {
                sample_rate_shading: true,
                wide_lines: true,
                fill_mode_non_solid: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("failed to create device");

    // --- queue ---
    let queue = queues.next().unwrap();

    // --- swapchain ---
    let (mut swapchain, images) = {
        let surface_capabilities = physical
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        let image_format = Some(
            physical
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count,

                image_format,
                image_extent: surface.window().inner_size().into(),

                image_usage: ImageUsage::color_attachment(),
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .iter()
                    .next()
                    .unwrap(),

                ..Default::default()
            },
        )
        .unwrap()
    };

    // --- render pass ---
    let render_pass = vulkano::single_pass_renderpass!(
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
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth}
        }
    )
    .unwrap();

    let size = window_size_u32(&surface);

    // --- buffers ---
    let vertex_buffer = gpu::immutable_buffer(
        init_data.combined_vertices.clone(),
        BufferUsage::vertex_buffer(),
        queue.clone(),
    );
    let index_buffer = gpu::immutable_buffer(
        init_data
            .all_faces
            .iter()
            .flatten()
            .flatten()
            .flat_map(|v| v.vertex_indices.clone())
            .collect::<Vec<_>>(),
        BufferUsage::index_buffer(),
        queue.clone(),
    );
    let mut uniform_buffers = gpu::uniform_buffers(&device, swapchain.image_count());
    let mut depth_buffer = AttachmentImage::with_usage(
        device.clone(),
        size,
        Format::D24_UNORM_S8_UINT,
        ImageUsage::depth_stencil_attachment(),
    )
    .unwrap();

    let debug_vertex_pool = CpuBufferPool::<Vertex>::vertex_buffer(device.clone());

    // --- shaders ---
    let vs = vert_shader::load(device.clone()).unwrap();
    let fs = frag_shader::load(device.clone()).unwrap();
    let dbg_vs = dbg_vert_shader::load(device.clone()).unwrap();
    let dbg_fs = dbg_frag_shader::load(device.clone()).unwrap();
    // TODO only activate when debugging

    // --- pipeline ---
    let pipeline = gpu::pipeline(
        device.clone(),
        vs.clone(),
        fs.clone(),
        render_pass.clone(),
        PrimitiveTopology::TriangleList,
        RasterizationState::new(),
        true,
    );
    let mut debug_rasterization_state = RasterizationState::new();
    debug_rasterization_state.line_width = StateMode::Fixed(5.0);

    let debug_pipeline = gpu::pipeline(
        device.clone(),
        dbg_vs.clone(),
        dbg_fs.clone(),
        render_pass.clone(),
        PrimitiveTopology::LineList,
        debug_rasterization_state,
        false,
    );

    // --- framebuffers ---
    let mut framebuffers = gpu::framebuffers(&images, render_pass.clone(), depth_buffer.clone());
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed_send_sync());

    // --- loop ---

    let (tx_render, rx_render) = oneshot::channel();
    let (tx_game, rx_game) = oneshot::channel();

    let (tx_system, rx_system) = oneshot::channel();

    let (tx_stop, rx_stop) = watch::channel(false);

    // actor
    handle.spawn_blocking(|| {
        let system = System::new();

        system.block_on(async {
            let camera = Camera::new().start();

            let renderer = Renderer::new(camera.clone()).start();

            let input_handler = InputHandler::new(camera.clone().recipient()).start();

            let game = Game::new(vec![camera.clone().recipient()]).start();

            tx_render
                .send((input_handler.clone(), renderer.clone(), camera.clone()))
                .unwrap();

            tx_game.send(game.clone()).unwrap();
        });

        tx_system.send(System::current()).unwrap();

        system.run().unwrap();
    });

    // game
    let system = rx_system.blocking_recv().unwrap();

    let (input_handler, renderer, camera) = rx_render.blocking_recv().unwrap();
    let game = rx_game.blocking_recv().unwrap();

    let g_rx_stop = rx_stop.clone();

    let g_camera = camera.clone();
    let g_game_recipient_load = game.clone().recipient();
    let g_game_recipient_tick = game.clone().recipient();

    let range = 2;

    let mut game_handle = Some(thread::spawn(move || {
        let rt = Runtime::new().unwrap();

        rt.block_on(async move {
            let mut game_ticker = interval(TICK_INTERVAL);
            let mut start = None;
            let mut counter = 0;

            while !*g_rx_stop.borrow() {
                let pos = g_camera.send(CameraPosRequest).await.unwrap().0;
                let pos = Camera::camera_pos_to_real(pos);

                g_game_recipient_load
                    .send(LoadChunkRange(
                        pos - Pos(range, range),
                        pos + Pos(range, range),
                    ))
                    .await
                    .unwrap();

                game_tick(
                    &g_game_recipient_tick,
                    &mut game_ticker,
                    &mut counter,
                    &mut start,
                )
                .await;
            }
        });
    }));

    // render
    let e_rx_stop = rx_stop.clone();

    let e_handle = handle.clone();

    let e_game = game.clone();

    event_loop.run(move |event, _, control_flow| {
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = &event
        {
            tx_stop.send(true).unwrap();

            game_handle
                .take()
                .expect("failed to take game_handle")
                .join()
                .expect("failed to close game thread");

            system.stop();

            *control_flow = ControlFlow::Exit;

            return;
        }

        if !*e_rx_stop.borrow() {
            let mut window_event = None;
            let mut device_event = None;

            match &event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => {
                    *&mut recreate_swapchain = true;
                }
                Event::WindowEvent { event, .. } => {
                    window_event = Some(event);
                }
                Event::DeviceEvent { event, .. } => {
                    device_event = Some(event);
                }
                Event::RedrawEventsCleared => {
                    let (width, height) = gpu::window_size(&surface);
                    let size = point2(width, height);
                    let aspect = width / height;

                    let draw_info = e_handle.block_on(renderer.send(Redraw { aspect })).unwrap();

                    let pos = Camera::camera_pos_to_real(draw_info.pos);

                    let world_render_context = e_handle
                        .block_on(e_game.send(WorldRenderContextRequest { pos, range }))
                        .unwrap();

                    let visible_chunks = world_render_context.visible_chunks;

                    let mut instance_buffer = visible_chunks
                        .iter()
                        .flat_map(|chunk| chunk.to_instances(&init_data))
                        .collect::<Vec<_>>();

                    let cursor_state = e_handle
                        .block_on(input_handler.send(CursorStateRequest))
                        .unwrap();

                    let camera_ray = e_handle
                        .block_on(camera.send(CameraRayRequest {
                            aspect,
                            size,
                            pos: cursor_state.pos,
                            visible_chunks: visible_chunks.clone(),
                            init_data: init_data.clone(),
                        }))
                        .unwrap();

                    camera_ray
                        .result
                        .into_iter()
                        .for_each(|(chunk, index, tile)| {
                            instance_buffer.push(
                                chunk
                                    .tile_to_instance(index, &tile, &init_data)
                                    .add_position_offset([0.0, 0.0, 0.001])
                                    .color_offset([1.0, 0.745, 0.447, 0.5]),
                            );
                        });

                    let debug = [];

                    let debug_vertex_buffer =
                        debug_vertex_pool.chunk(debug).unwrap().into_buffer_slice();

                    render(
                        &draw_info,
                        device.clone(),
                        queue.clone(),
                        surface.clone(),
                        render_pass.clone(),
                        &mut swapchain,
                        &mut recreate_swapchain,
                        &mut framebuffers,
                        &mut previous_frame_end,
                        pipeline.clone(),
                        vertex_buffer.clone(),
                        index_buffer.clone(),
                        debug_pipeline.clone(),
                        debug_vertex_buffer.clone(),
                        &mut depth_buffer,
                        &mut uniform_buffers,
                        &instance_buffer,
                        &init_data,
                    );
                }
                _ => (),
            };

            if window_event.is_some() || device_event.is_some() {
                e_handle
                    .block_on(input_handler.send(convert_input(window_event, device_event)))
                    .unwrap();
            }
        }
    });
}
