use env_logger::Env;
use expect_dialog::ExpectDialog;
use futures::executor::block_on;
use tokio::runtime::Runtime;
use winit::event_loop::EventLoop;
use winit::window::{Icon, WindowBuilder};

use automancy::gpu::{Gpu, DEPTH_FORMAT};
use automancy::renderer::Renderer;
use automancy_defs::gui::init_gui;
use automancy_defs::log;

use crate::event::{on_event, EventLoopStorage};
use crate::setup::GameSetup;

pub static LOGO: &[u8] = include_bytes!("assets/logo.png");

mod event;
mod gui;
mod setup;

/// Gets the game icon.
fn get_icon() -> Icon {
    let image = image::load_from_memory(LOGO).unwrap().to_rgba8();
    let width = image.width();
    let height = image.height();

    Icon::from_rgba(image.into_flat_samples().samples, width, height).unwrap() // unwrap ok
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let runtime = Runtime::new().unwrap();

    // --- window ---
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("automancy")
        .with_window_icon(Some(get_icon()))
        .build(&event_loop)
        .expect_dialog("Failed to open window!");

    // --- setup ---
    let (mut setup, vertices, indices) = runtime
        .block_on(GameSetup::setup(&window))
        .expect_dialog("Critical failure in game setup!");

    // --- render ---
    log::info!("setting up rendering...");
    let gpu = block_on(Gpu::new(window, vertices, indices));
    log::info!("render setup.");

    // --- gui ---
    log::info!("setting up gui...");
    let mut gui = init_gui(
        egui_wgpu::Renderer::new(&gpu.device, gpu.config.format, Some(DEPTH_FORMAT), 4),
        &gpu.window,
    );
    log::info!("gui set up.");

    let mut renderer = Renderer::new(setup.resource_man.clone(), gpu);

    let mut storage = EventLoopStorage::default();

    event_loop.run(move |event, _, control_flow| {
        let _ = on_event(
            &runtime,
            &mut setup,
            &mut storage,
            &mut renderer,
            &mut gui,
            event,
            control_flow,
        );
    });
}
