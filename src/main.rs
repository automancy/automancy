#![feature(unzip_option)]
#![feature(split_array)]
#![feature(array_chunks)]
#![feature(slice_group_by)]
#![feature(duration_consts_float)]
#![feature(variant_count)]

#[macro_use]
mod util;
#[macro_use]
mod game;
mod math;
mod registry;

use crate::{
    game::{
        data::{id::RawId, map::Map, tile::TileCoord},
        game::Game,
        player::input::{handler::InputHandler, primitive::convert_input},
        render::{
            camera::Camera,
            data::InstanceData,
            gpu::{self, dbg_frag_shader, dbg_vert_shader, frag_shader, vert_shader},
            renderer::Renderer,
        },
        ticking::{TICK_INTERVAL, TPS},
    },
    math::{
        cg::Num,
        hex::{cube::CubeCoord, offset::OffsetCoord},
    },
};

use cgmath::point2;
use game::{
    data::id::Id,
    game::Ticked,
    render::{
        data::{Model, UniformBufferObject, Vertex},
        gpu::window_size_u32,
    },
    ticking::MAX_ALLOWED_TICK_INTERVAL,
};
use json::JsonValue;
use math::cg::{DisplayCoord, Matrix4};
use registry::init::InitData;
use util::resource::{Resource, ResourceManager};
use walkdir::WalkDir;

use std::{
    borrow::Borrow,
    ffi::OsStr,
    fs::{read_to_string, File},
    path::Path,
    sync::Arc,
    thread,
    time::Instant,
};

use tokio::{
    runtime::{Builder, Runtime},
    sync::{
        broadcast::{channel, Sender},
        oneshot, OnceCell,
    },
    time::interval,
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
        graphics::{input_assembly::PrimitiveTopology, rasterization::RasterizationState},
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

fn load_resources() -> (ResourceManager, Vec<Option<(Id, Option<Model>)>>) {
    let mut resource_man = ResourceManager::default();

    // TODO: just use serde?

    let resources = WalkDir::new(RESOURCE)
        .into_iter()
        .flatten()
        .map(|entry| {
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

            return None;
        })
        .flatten()
        .map(|(json, working_dir)| {
            return resource_man.load_resource(json, &working_dir);
        })
        .collect::<Vec<_>>();

    println!("{:?}", resources);

    (resource_man, resources)
}

fn render(
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
    matrix: Matrix4,
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
        matrix: matrix.into(),
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

pub async fn game_tick(game: &mut Game, counter: &mut u64, cycle_start: &mut Option<Instant>) {
    let start = Instant::now();
    game.tick();
    let finish = Instant::now();

    let tick_time = finish - start;

    if tick_time >= MAX_ALLOWED_TICK_INTERVAL {
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
            if let Some(start) = cycle_start.replace(Instant::now()) {
                let elapsed = start.elapsed();

                let tps = (*counter * 1_000_000_000) as f64 / elapsed.as_nanos() as f64;
                let tps = (tps * 100.0).round() / 100.0;

                log::debug!("TPS: {:.1}, elapsed time: {:?}", tps, elapsed);
            }
        }

        *counter -= TPS;
    }
}

async fn init() -> Arc<InitData> {
    let (resource_man, resources) = load_resources();

    let init_data = InitData::new(resource_man, resources);

    Arc::new(init_data)
}

fn coord_to_instance(
    map_ref: &Map,
    init_data: &InitData,
    pos: TileCoord,
    none: Option<Id>,
) -> Option<InstanceData> {
    let map_id_pool = &map_ref.id_pool;
    let resource_id_pool = &init_data.resource_man.id_pool;

    map_ref
        .tiles
        .get(&pos)
        .map_or(none, |tile| {
            tile.id
                .to_raw_id(map_id_pool)
                .to_id(resource_id_pool)
                .as_ref()
                .copied()
        })
        .and_then(|id| init_data.resource_man.resources.get(&id))
        .and_then(|v| {
            v.faces_index.map(|face| {
                // TODO into

                let pos = CubeCoord::new(pos.q() as Num, pos.r() as Num);
                let pos = DisplayCoord::from_cube_as_pointy_top(&pos).to_point2();

                InstanceData::new()
                    .faces_index(face)
                    .position_offset([pos.x, pos.y, 0.0])
            })
        })
}

fn main() {
    env_logger::init();

    // --- resources & data ---
    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
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

    // --- load map ---

    let map = Map::load("test".to_owned());

    // --- loop ---
    let (send_stop, recv_stop) = watch::channel(false);

    let (game, recv_game_state) = Game::new(map);

    let (send_map_ref, recv_map_ref) = oneshot::channel();

    let (mut input_handler, recv_input_state) = InputHandler::new();

    let (mut renderer, recv_renderer_state) = Renderer::new(recv_input_state.clone());

    let (camera, recv_camera_state) = Camera::new(
        recv_input_state.clone(),
        recv_game_state.resubscribe(),
        recv_renderer_state.resubscribe(),
    );

    // game
    {
        let recv_stop = recv_stop.clone();

        thread::spawn(move || {
            let mut game = game;
            let mut camera = camera;

            send_map_ref.send(game.map_ref()).unwrap();

            Runtime::new().unwrap().block_on(async move {
                let mut ticker = interval(TICK_INTERVAL);
                let mut start = Some(Instant::now());
                let mut counter = 0;

                while false == *recv_stop.borrow() {
                    game_tick(&mut game, &mut counter, &mut start).await;

                    camera.recv().await;
                    camera.send();

                    ticker.tick().await;
                }
            });
        });
    }

    let map_ref = recv_map_ref.blocking_recv().unwrap();

    {
        let pos = CubeCoord::new(0 as Num, 0 as Num);
        let pos = DisplayCoord::from_cube_as_pointy_top(&pos).to_point2();

        println!("{:?}", pos)
    }

    {
        let pos = CubeCoord::new(1 as Num, 1 as Num);
        let pos = DisplayCoord::from_cube_as_pointy_top(&pos).to_point2();

        println!("{:?}", pos)
    }

    {
        let recv_stop = recv_stop.clone();
        let mut recv_camera_state = recv_camera_state.resubscribe();
        let handle = handle.clone();

        // render
        event_loop.run(move |event, _, control_flow| {
            if true == *recv_stop.borrow() {
                return;
            }

            let mut window_event = None;
            let mut device_event = None;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;

                    send_stop.send(true).unwrap();

                    return;
                }

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
                    let window_size = point2(width, height);
                    let aspect = width / height;

                    renderer.update(aspect, window_size);
                    renderer.recv();
                    renderer.send();

                    let camera_state = handle.block_on(recv_camera_state.recv()).unwrap(); // TODO unwrap

                    let none = init_data.resource_man.id_pool.id(&RawId::none());

                    let instance_buffer = {
                        let pos = Camera::point3_to_tile_coord(camera_state.pos);

                        // TODO move this constant
                        const RANGE: isize = 32;
                        let point = TileCoord::new(RANGE, RANGE);

                        let min = pos - point;
                        let max = pos + point;
                        let q = min.q()..max.q();
                        let r = min.r()..max.r();

                        let iter = q
                            .into_iter()
                            .map(|q| r.clone().into_iter().map(move |r| (q, r)))
                            .flatten();

                        // TODO make conversion pool

                        let map_ref = map_ref.lock().unwrap();

                        iter.map(|(q, r)| TileCoord::new(q, r))
                            .map(|coord| coord_to_instance(&map_ref, &init_data, coord, none))
                            .chain([coord_to_instance(
                                &map_ref,
                                &init_data,
                                camera_state.pointing_at,
                                none,
                            )
                            .map(|instance| {
                                instance
                                    .add_position_offset([0.0, 0.0, 0.001])
                                    .color_offset([1.0, 0.745, 0.447, 0.5])
                            })])
                            .flatten()
                            .collect::<Vec<_>>()
                    };

                    let debug = [];

                    let debug_vertex_buffer =
                        debug_vertex_pool.chunk(debug).unwrap().into_buffer_slice();

                    render(
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
                        camera_state.matrix,
                    );
                }
                _ => (),
            };

            if window_event.is_some() || device_event.is_some() {
                input_handler.send(convert_input(window_event, device_event));
            }
        });
    }
}
