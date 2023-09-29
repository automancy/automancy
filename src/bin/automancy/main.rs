use std::fmt::Write;
use std::fs::File;
use std::panic::PanicInfo;
use std::path::Path;
use std::{env, panic};

use color_eyre::config::HookBuilder;
use color_eyre::eyre;
use color_eyre::owo_colors::OwoColorize;
use env_logger::Env;
use futures::executor::block_on;
use native_dialog::{MessageDialog, MessageType};
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Fullscreen, Icon, WindowBuilder};

use automancy::camera::Camera;
use automancy::gpu::Gpu;
use automancy::input::KeyActions;
use automancy_defs::gui::init_gui;
use automancy_defs::{log, window};

use crate::event::{on_event, EventLoopStorage};
use crate::renderer::Renderer;
use crate::setup::GameSetup;

pub static LOGO: &[u8] = include_bytes!("assets/logo.png");

mod event;
mod gui;
pub mod renderer;
mod setup;

/// Gets the game icon.
fn get_icon() -> Icon {
    let image = image::load_from_memory(LOGO).unwrap().to_rgba8();
    let width = image.width();
    let height = image.height();

    Icon::from_rgba(image.into_flat_samples().samples, width, height).unwrap() // unwrap ok
}

fn write_msg<P: AsRef<Path>>(buffer: &mut impl Write, file_path: P) -> std::fmt::Result {
    writeln!(buffer, "Well, this is embarrassing.\n")?;
    writeln!(
        buffer,
        "automancy had a problem and crashed. To help us diagnose the problem you can send us a crash report.\n"
    )?;
    writeln!(
        buffer,
        "We have generated a report file at\nfile://{}\n\nSubmit an issue or tag us on Fedi/Discord and include the report as an attachment.\n",
        file_path.as_ref().display(),
    )?;

    writeln!(
        buffer,
        "- Git: https://github.com/sorcerers-class/automancy"
    )?;
    writeln!(buffer, "- Fedi(Mastodon): https://gamedev.lgbt/@automancy")?;
    writeln!(buffer, "- Discord: https://discord.gg/jcJbUh3QX2")?;

    writeln!(
        buffer,
        "\nAlternatively, send an email to the main developer Madeline Sparkles (madeline@mouse.lgbt) directly.\n"
    )?;

    writeln!(
        buffer,
        "We take privacy seriously, and do not perform any automated error collection. In order to improve the software, we rely on people to submit reports.\n"
    )?;
    writeln!(buffer, "Thank you kindly!")?;

    Ok(())
}
fn main() -> eyre::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    {
        let eyre = HookBuilder::blank()
            .capture_span_trace_by_default(true)
            .display_env_section(false);

        let (panic_hook, eyre_hook) = eyre.into_hooks();

        eyre_hook.install()?;

        panic::set_hook(Box::new(move |info: &PanicInfo| {
            let file_path = {
                let report = panic_hook.panic_report(info);

                let uuid = Uuid::new_v4().hyphenated().to_string();
                let tmp_dir = env::temp_dir();
                let file_name = format!("automancy-report-{uuid}.txt");
                let file_path = tmp_dir.join(file_name);
                if let Ok(mut file) = File::create(&file_path) {
                    use std::io::Write;

                    _ = write!(
                        file,
                        "{}",
                        strip_ansi_escapes::strip_str(report.to_string())
                    );
                }
                eprintln!("{}", report);

                file_path
            };

            if let Some(location) = info.location() {
                if !["src/game.rs", "src/tile_entity.rs"].contains(&location.file()) {
                    let message = {
                        let mut message = String::new();
                        _ = write_msg(&mut message, &file_path);

                        message
                    };

                    {
                        eprintln!("\n\n\n{}\n\n\n", message.bright_red());

                        _ = MessageDialog::new()
                            .set_type(MessageType::Error)
                            .set_title("automancy crash dialog")
                            .set_text(&message)
                            .show_alert();
                    }
                }
            }
        }));
    }

    // --- window ---
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("automancy")
        .with_window_icon(Some(get_icon()))
        .with_min_inner_size(PhysicalSize::new(200, 200))
        .build(&event_loop)
        .expect("failed to open window!");

    let camera = Camera::new(window::window_size_double(&window));

    // --- setup ---
    let runtime = Runtime::new().unwrap();

    let (mut setup, vertices, indices) = runtime
        .block_on(GameSetup::setup(camera))
        .expect("Critical failure in game setup!");

    // --- render ---
    log::info!("Setting up rendering...");
    let gpu = block_on(Gpu::new(
        window,
        &setup.resource_man,
        vertices,
        indices,
        setup.options.graphics.fps_limit == 0.0,
    ));
    log::info!("Render setup.");

    // --- gui ---
    log::info!("Setting up gui...");
    let mut gui = init_gui(
        egui_wgpu::Renderer::new(&gpu.device, gpu.config.format, None, 1),
        &gpu.window,
    );
    log::info!("Gui set up.");

    let mut renderer = Renderer::new(gpu);

    let mut storage = EventLoopStorage::default();

    let mut closed = false;

    event_loop.run(move |event, _, control_flow| {
        if closed {
            return;
        }

        match on_event(
            &mut setup,
            &mut storage,
            &mut renderer,
            &mut gui,
            event,
            control_flow,
        ) {
            Ok(to_exit) => {
                if to_exit {
                    closed = true;
                }
            }
            Err(e) => {
                log::warn!("Event loop returned error: {e}");
            }
        }

        renderer
            .gpu
            .set_vsync(setup.options.graphics.fps_limit == 0.0);

        setup.options.graphics.fullscreen = setup.input_handler.key_active(KeyActions::Fullscreen);
        if setup.options.graphics.fullscreen {
            renderer
                .gpu
                .window
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            renderer.gpu.window.set_fullscreen(None);
        }
    });
}
