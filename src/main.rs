#![feature(split_array)]
#![feature(array_chunks)]
#![feature(slice_group_by)]
#![feature(trivial_bounds)]
#![feature(trait_alias)]

use std::{
    collections::HashMap,
    fs::File,
    sync::{Arc, Mutex},
};

use cgmath::{point3, Rad};

use registry::init::InitData;

use game::{
    player::control::{MainClickListener, MainHoldListener, MainMoveListener},
    render::data::{UniformBufferObject, Vertex},
};
use tokio::{
    sync::OnceCell,
    time::{self, Instant, Interval},
};

#[macro_use]
mod util;

mod game;
mod math;
mod registry;

use util::resource::Resource;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, ImmutableBuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo},
    image::ImageUsage,
    instance::{Instance, InstanceCreateInfo},
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline, Pipeline, PipelineBindPoint},
    render_pass::{Framebuffer, RenderPass},
    swapchain::{
        acquire_next_image, AcquireError, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, Window, WindowBuilder},
};

use crate::game::{
    data::{
        chunk::{Chunk, DataPool, IdPool, Tiles},
        id::Id,
        pos::Pos,
    },
    game::Game,
    player::player::Player,
    render::{
        camera::Camera,
        data::InstanceData,
        gpu::{self, fs, vs},
    },
    ticking::{TICK_INTERVAL, TPS},
};

pub const ASSET_LOGO: &str = "assets/logo.png";

// TODO: use metadata file + directory scan
pub const RESOURCE_TILE: &str = "resources/tile.ply";
pub const RESOURCE_TILE_PURPLE: &str = "resources/tile_purple.ply";

pub type Re = OnceCell<Resource>;

async fn load_resources() -> Vec<(&'static str, Resource)> {
    resolve_all! {
        RESOURCE_TILE,
        RESOURCE_TILE_PURPLE
    }
}

fn render(
    game: &Game,
    player: &Player,
    camera: &Arc<Mutex<Camera>>,
    device: &Arc<Device>,
    queue: &Arc<Queue>,
    surface: &Surface<Window>,
    viewport: &mut Viewport,
    render_pass: &Arc<RenderPass>,
    swapchain: &mut Arc<Swapchain<Window>>,
    recreate_swapchain: &mut bool,
    framebuffers: &mut Vec<Arc<Framebuffer>>,
    previous_frame_end: &mut Option<Box<dyn GpuFuture>>,
    pipeline: &Arc<GraphicsPipeline>,
    vertex_buffer: &Arc<ImmutableBuffer<[Vertex]>>,
    index_buffer: &Arc<ImmutableBuffer<[u32]>>,
    uniform_buffers: &mut Vec<Arc<CpuAccessibleBuffer<UniformBufferObject>>>,
    instance_buffer: &[InstanceData],
    init_data: &InitData,
) {
    let dimensions = surface.window().inner_size();
    if dimensions.width == 0 || dimensions.height == 0 {
        return;
    }
    previous_frame_end.as_mut().unwrap().cleanup_finished();

    if *recreate_swapchain {
        let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
            image_extent: dimensions.into(),
            ..swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("failed to recreate swapchain: {:?}", e),
        };

        *swapchain = new_swapchain;
        // Because framebuffers contains an Arc on the old swapchain, we need to
        // recreate framebuffers as well.
        *framebuffers = gpu::framebuffers(&new_images, render_pass.clone());
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

    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    let (instance_buffer, indirect_commands) =
        gpu::indirect_buffer(queue.clone(), init_data, instance_buffer);

    let indirect_buffer = gpu::immutable_buffer(
        indirect_commands.into_iter(),
        BufferUsage::indirect_buffer(),
        queue.clone(),
    );

    let camera = camera.lock().unwrap();

    let ubo = UniformBufferObject {
        view: camera.view().into(),
        //projection: camera.projection().into(),
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

    builder
        .begin_render_pass(
            framebuffers[image_num].clone(),
            SubpassContents::Inline,
            vec![[0.0, 0.0, 0.0, 1.0].into()],
        )
        .unwrap()
        .set_viewport(0, [viewport.clone()])
        .bind_pipeline_graphics(pipeline.clone())
        .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer.clone()))
        .bind_index_buffer(index_buffer.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            pipeline_layout.clone(),
            0,
            ubo_set,
        )
        .draw_indexed_indirect(indirect_buffer)
        .unwrap()
        .end_render_pass()
        .unwrap();

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
            *previous_frame_end = Some(future.boxed());
        }
        Err(FlushError::OutOfDate) => {
            *recreate_swapchain = true;
            *previous_frame_end = Some(sync::now(device.clone()).boxed());
        }
        Err(e) => {
            log::error!("failed to flush future: {:?}", e);
            *previous_frame_end = Some(sync::now(device.clone()).boxed());
        }
    }
}

fn handle_window(
    game: &Game,
    player: &Player,
    camera: &Arc<Mutex<Camera>>,
    event: Event<()>,
    control_flow: &mut ControlFlow,
    device: &Arc<Device>,
    queue: &Arc<Queue>,
    surface: &Surface<Window>,
    viewport: &mut Viewport,
    render_pass: &Arc<RenderPass>,
    swapchain: &mut Arc<Swapchain<Window>>,
    recreate_swapchain: &mut bool,
    framebuffers: &mut Vec<Arc<Framebuffer>>,
    previous_frame_end: &mut Option<Box<dyn GpuFuture>>,
    pipeline: &Arc<GraphicsPipeline>,
    vertex_buffer: &Arc<ImmutableBuffer<[Vertex]>>,
    index_buffer: &Arc<ImmutableBuffer<[u32]>>,
    uniform_buffers: &mut Vec<Arc<CpuAccessibleBuffer<UniformBufferObject>>>,
    instance_buffer: &[InstanceData],
    init_data: &mut InitData,
    main_hold_listeners: &mut Vec<Arc<Mutex<dyn MainHoldListener>>>,
    main_click_listeners: &mut Vec<Arc<Mutex<dyn MainClickListener>>>,
    main_move_listeners: &mut Vec<Arc<Mutex<dyn MainMoveListener>>>,
) {
    let mut window_event = None;
    let mut device_event = None;

    match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            *recreate_swapchain = true;
        }
        Event::RedrawEventsCleared => render(
            game,
            player,
            camera,
            device,
            queue,
            surface,
            viewport,
            render_pass,
            swapchain,
            recreate_swapchain,
            framebuffers,
            previous_frame_end,
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffers,
            instance_buffer,
            init_data,
        ),
        Event::WindowEvent { event, .. } => {
            window_event = Some(event);
        }
        Event::DeviceEvent { event, .. } => {
            device_event = Some(event);
        }
        _ => (),
    };

    game.window_event(
        window_event,
        device_event,
        init_data,
        main_hold_listeners,
        main_click_listeners,
        main_move_listeners,
    );
}

pub async fn main_game_tick(game_time: Instant) {
    //time::sleep(Duration::from_millis(100)).await;
}

pub async fn game_tick(
    counter: &mut u64,
    start: &mut Option<Instant>,
    game_ticker: &mut Interval,
) -> Option<()> {
    let game_time = game_ticker.tick().await;

    let game_tick_start = Instant::now();
    main_game_tick(game_time).await;
    let game_tick_end = Instant::now();
    let tick_time = game_tick_end.duration_since(game_tick_start);

    if tick_time > TICK_INTERVAL {
        log::warn!(
            "tick took longer than allowed maximum! tick_time: {:?}, maximum: {:?}",
            tick_time,
            TICK_INTERVAL
        );
    }

    *counter += 1;
    if *counter >= TPS {
        // todo: what to do when the TPS is low?
        {
            if let Some(start) = start.take() {
                let elapsed = start.elapsed();
                let tps = (*counter * 1_000_000_000) as f64 / elapsed.as_nanos() as f64;
                let tps = (tps * 100.0).round() / 100.0;

                log::debug!("[game_ticker] TPS: {:.1}, elapsed time: {:?}", tps, elapsed);
            }
            start.replace(Instant::now());
        }

        *counter -= TPS;
    }
    Some(())
}

#[tokio::main]
async fn main() {
    env_logger::init();

    // --- resources & data ---

    let resources = load_resources().await;

    let mut init_data = InitData::new(resources);

    let mut id_pool: IdPool = IdPool::default();
    id_pool.data[0] = Id::new("automancy".to_string(), "uwu".to_string());

    let chunk = Chunk {
        chunk_pos: Pos(0, 0),
        data_pool: DataPool::default(),
        id_pool,
        tiles: Tiles::default(),
    };

    chunk.unload();

    let chunk = Chunk::load(Pos(0, 0));

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
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    )
    .unwrap();

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
            .flat_map(|v| v.iter().flat_map(|v| v.vertex_indices.clone()))
            .collect::<Vec<_>>(),
        BufferUsage::index_buffer(),
        queue.clone(),
    );
    let mut uniform_buffers = gpu::uniform_buffers(&device, swapchain.image_count());

    // --- shaders ---
    let vs = vs::load(device.clone()).unwrap();
    let fs = fs::load(device.clone()).unwrap();

    // --- pipeline ---
    let pipeline = gpu::pipeline(device.clone(), vs.clone(), fs.clone(), render_pass.clone());

    // --- viewport ---
    let mut viewport = gpu::viewport();

    // --- framebuffers ---
    let mut framebuffers = gpu::framebuffers(&images, render_pass.clone());
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    // --- loop ---

    // game
    let pos = Arc::new(Mutex::new(point3(0.0, 0.0, 1.0)));

    let mut player = Player { pos: pos.clone() };

    let mut camera = Arc::new(Mutex::new(Camera {
        pos: pos.clone(),
        rotation: Rad(0.0),
        holding_main: false,
    }));

    let game = Game {
        loaded_chunks: HashMap::new(),
    };

    let mut main_hold_listeners: Vec<Arc<Mutex<dyn MainHoldListener>>> = vec![camera.clone()];
    let mut main_click_listeners: Vec<Arc<Mutex<dyn MainClickListener>>> = vec![];
    let mut main_move_listeners: Vec<Arc<Mutex<dyn MainMoveListener>>> = vec![camera.clone()];

    tokio::spawn(async {
        let mut game_ticker = time::interval(TICK_INTERVAL);
        let mut start = None;
        let mut counter = 0;

        loop {
            game_tick(&mut counter, &mut start, &mut game_ticker).await;
        }
    });

    // render
    event_loop.run(move |event, _, control_flow| {
        let instance_buffer = [
            InstanceData {
                position_offset: [-0.5, 0.0, 0.0],
                scale: 0.5,
                faces_index: init_data.resources_map[RESOURCE_TILE],
            },
            InstanceData {
                position_offset: [0.5, 0.0, 0.0],
                scale: 0.5,
                faces_index: init_data.resources_map[RESOURCE_TILE_PURPLE],
            },
        ];

        handle_window(
            &game,
            &player,
            &camera.clone(),
            event,
            control_flow,
            &device,
            &queue,
            &surface,
            &mut viewport,
            &render_pass,
            &mut swapchain,
            &mut recreate_swapchain,
            &mut framebuffers,
            &mut previous_frame_end,
            &pipeline,
            &vertex_buffer,
            &index_buffer,
            &mut uniform_buffers,
            &instance_buffer,
            &mut init_data,
            &mut main_hold_listeners,
            &mut main_click_listeners,
            &mut main_move_listeners,
        );
    });
}
