use egui::Frame;
use egui_winit_vulkano::Gui;
use flexstr::SharedStr;
use kira::manager::backend::cpal::CpalBackend;
use kira::manager::{AudioManager, AudioManagerSettings};
use kira::track::{TrackBuilder, TrackHandle};
use riker::actor::{ActorRef, ActorRefFactory};
use riker::actors::{ActorSystem, SystemBuilder, Timer};
use std::fs;
use std::sync::Arc;
use vulkano::device::DeviceExtensions;
use winit::event_loop::EventLoop;
use winit::window::{Icon, Window};

use crate::game::ticking::TICK_INTERVAL;
use crate::game::{Game, GameMsg};
use crate::render::camera::Camera;
use crate::render::gpu::{Gpu, RenderAlloc};
use crate::render::renderer::Renderer;
use crate::render::{gpu, gui};
use crate::resource::{ResourceManager, RESOURCES_FOLDER};
use crate::LOGO;

/// Stores what the game initializes on startup.
pub struct GameSetup {
    /// the audio manager
    pub(crate) audio_man: AudioManager,
    /// the resource manager
    pub(crate) resource_man: Arc<ResourceManager>,
    /// the GUI system
    pub(crate) gui: Gui,
    /// the Riker actor system
    pub(crate) sys: ActorSystem,
    /// the game messaging system
    pub(crate) game: ActorRef<GameMsg>,
    /// the egui frame
    pub(crate) frame: Frame,
    /// the renderer
    pub(crate) renderer: Renderer,
    /// the camera
    pub(crate) camera: Camera,
    /// the window
    pub(crate) window: Arc<Window>,
}

impl GameSetup {
    /// Initializes the game, filling all the necessary fields as well as creating an event loop.
    pub fn setup() -> (EventLoop<()>, Self) {
        // --- resources & data ---
        let mut audio_man =
            AudioManager::<CpalBackend>::new(AudioManagerSettings::default()).unwrap();
        let track = audio_man
            .add_sub_track({
                let builder = TrackBuilder::new();

                builder
            })
            .unwrap();
        log::info!("audio backend initialized");
        let resource_man = load_resources(track);
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
            gpu::get_physical_device(instance, surface.clone(), &device_extensions);
        log::info!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type
        );

        let (device, mut queues) = gpu::get_logical_device(
            physical_device.clone(),
            queue_family_index,
            device_extensions,
        );
        let queue = queues.next().unwrap();

        let alloc = RenderAlloc::new(
            resource_man.clone(),
            device.clone(),
            surface.clone(),
            window.clone(),
            physical_device,
        );
        let gpu = Gpu::new(device, queue, surface, window.clone(), alloc);

        let gui = gui::init_gui(&event_loop, &gpu);
        log::info!("Renderer setup complete");
        // --- setup game ---
        let sys = SystemBuilder::new().name("automancy").create().unwrap();

        // TODO map selection
        let map_name = SharedStr::from_static("test");

        let game = sys
            .actor_of_args::<Game, (Arc<ResourceManager>, SharedStr)>(
                "game",
                (resource_man.clone(), map_name),
            )
            .unwrap();
        game.send_msg(GameMsg::LoadMap(resource_man.clone()), None);

        sys.schedule(
            TICK_INTERVAL,
            TICK_INTERVAL,
            game.clone(),
            None,
            GameMsg::Tick,
        );

        log::info!("loading completed!");

        // last setup
        let frame = gui::default_frame();

        let renderer = Renderer::new(resource_man.clone(), gpu);
        let camera = Camera::default();

        // --- event-loop ---
        (
            event_loop,
            GameSetup {
                audio_man,
                resource_man,
                gui,
                sys,
                game,
                frame,
                renderer,
                camera,
                window,
            },
        )
    }
}

/// Initialize the Resource Manager system, and loads all the resources in all namespaces.
fn load_resources(track: TrackHandle) -> Arc<ResourceManager> {
    let mut resource_man = ResourceManager::new(track);

    fs::read_dir(RESOURCES_FOLDER)
        .unwrap()
        .flatten()
        .map(|v| v.path())
        .for_each(|dir| {
            let namespace = dir.file_name().unwrap().to_str().unwrap();
            log::info!("loading namespace {namespace}");
            resource_man.load_models(&dir);
            resource_man.load_scripts(&dir);
            resource_man.load_translates(&dir);
            resource_man.load_audio(&dir);
            resource_man.load_functions(&dir);
            resource_man.load_items(&dir);
            resource_man.load_tags(&dir);
            resource_man.load_tiles(&dir);
            log::info!("finished loading namespace {namespace}");
        });

    resource_man.ordered_items();
    resource_man.compile_models();

    Arc::new(resource_man)
}

/// Gets the game icon.
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
