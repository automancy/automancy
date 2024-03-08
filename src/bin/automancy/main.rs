#![windows_subsystem = "windows"]

use std::fmt::Write;
use std::fs::File;
use std::panic::PanicInfo;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, panic};

use color_eyre::config::HookBuilder;
use color_eyre::eyre;
use egui::{FontData, FontDefinitions};
use env_logger::Env;
use num::Zero;
use rfd::{MessageButtons, MessageDialog, MessageLevel};
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Fullscreen, Icon, WindowBuilder};

use automancy::camera::Camera;
use automancy::event::{on_event, EventLoopStorage};
use automancy::game::load_map;
use automancy::gpu::{init_gpu_resources, Gpu, DEPTH_FORMAT};
use automancy::map::MAIN_MENU;
use automancy::renderer::Renderer;
use automancy::setup::GameSetup;
use automancy::LOGO;
use automancy_defs::flexstr::ToSharedStr;
use automancy_defs::gui::init_gui;
use automancy_defs::gui::set_font;
use automancy_defs::math::Double;
use automancy_defs::{log, window};
use automancy_resources::kira::tween::Tween;

/// Gets the game icon.
fn get_icon() -> Icon {
    let image = image::load_from_memory(LOGO).unwrap().to_rgba8();
    let width = image.width();
    let height = image.height();

    let samples = image.into_flat_samples().samples;
    Icon::from_rgba(samples, width, height).unwrap()
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

    writeln!(buffer, "- Git: https://github.com/automancy/automancy")?;
    writeln!(buffer, "- Fedi(Mastodon): https://gamedev.lgbt/@automancy")?;
    writeln!(buffer, "- Discord: https://discord.gg/ee9XebxNaa")?;

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
                        eprintln!("\n\n\n{}\n\n\n", message);

                        _ = MessageDialog::new()
                            .set_level(MessageLevel::Error)
                            .set_buttons(MessageButtons::Ok)
                            .set_title("automancy crash dialog")
                            .set_description(message)
                            .show();
                    }
                }
            }
        }));
    }

    // --- window ---
    let event_loop = EventLoop::new()?;

    let icon = get_icon();

    let window = WindowBuilder::new()
        .with_title("automancy")
        .with_window_icon(Some(icon))
        .with_min_inner_size(PhysicalSize::new(200, 200))
        .build(&event_loop)
        .expect("Failed to open window");

    let camera = Camera::new(window::window_size_double(&window));

    // --- setup ---
    let runtime = Runtime::new().unwrap();

    let (mut setup, vertices, indices) = runtime
        .block_on(GameSetup::setup(camera))
        .expect("Critical failure in game setup");

    let gpu = runtime.block_on(Gpu::new(&window, setup.options.graphics.fps_limit == 0.0));

    // --- gui ---
    log::info!("Setting up gui...");
    let mut egui_renderer =
        egui_wgpu::Renderer::new(&gpu.device, gpu.config.format, Some(DEPTH_FORMAT), 4);
    let egui_callback_resources = &mut egui_renderer.callback_resources;
    egui_callback_resources.insert(setup.start_instant);
    egui_callback_resources.insert(setup.resource_man.clone());

    // - render -
    log::info!("Setting up rendering...");
    let (shared_resources, render_resources, global_buffers, gui_resources) = init_gpu_resources(
        &gpu.device,
        &gpu.queue,
        &gpu.config,
        &setup.resource_man,
        vertices,
        indices,
    );
    let global_buffers = Arc::new(global_buffers);
    log::info!("Render setup.");
    // - render -

    egui_callback_resources.insert(gui_resources);
    egui_callback_resources.insert(global_buffers.clone());

    let mut gui = init_gui(egui_renderer, gpu.window);
    gui.fonts = FontDefinitions::default();
    for (name, font) in setup.resource_man.fonts.iter() {
        gui.fonts
            .font_data
            .insert(name.to_string(), FontData::from_owned(font.data.clone()));
    }
    set_font(setup.options.gui.font.to_shared_str(), &mut gui);
    log::info!("Gui set up.");

    let mut renderer = Renderer::new(
        gpu,
        shared_resources,
        render_resources,
        global_buffers.clone(),
        &setup.options,
    );
    let mut loop_store = EventLoopStorage::default();
    let mut closed = false;

    // load the main menu
    runtime
        .block_on(load_map(&setup, &mut loop_store, MAIN_MENU.to_string()))
        .unwrap();

    event_loop.run(move |event, target| {
        if closed {
            return;
        }

        match on_event(
            &runtime,
            &mut setup,
            &mut loop_store,
            &mut renderer,
            &mut gui,
            event,
            target,
        ) {
            Ok(to_exit) => {
                if to_exit {
                    closed = true;
                    return;
                }
            }
            Err(e) => {
                log::warn!("Event loop returned error: {e}");
            }
        }

        if !setup.options.synced {
            gui.context.set_zoom_factor(setup.options.gui.scale);
            set_font(setup.options.gui.font.to_shared_str(), &mut gui);

            setup
                .audio_man
                .main_track()
                .set_volume(setup.options.audio.sfx_volume, Tween::default())
                .unwrap();

            renderer
                .gpu
                .set_vsync(setup.options.graphics.fps_limit == 0.0);

            if setup.options.graphics.fps_limit >= 250.0 {
                renderer.fps_limit = Double::INFINITY;
            } else {
                renderer.fps_limit = setup.options.graphics.fps_limit;
            }

            if setup.options.graphics.fullscreen {
                renderer
                    .gpu
                    .window
                    .set_fullscreen(Some(Fullscreen::Borderless(None)));
            } else {
                renderer.gpu.window.set_fullscreen(None);
            }

            setup.options.synced = true;
        }

        if !renderer.fps_limit.is_zero() {
            let frame_time = Duration::from_secs_f64(1.0 / renderer.fps_limit);

            if loop_store.frame_start.elapsed() > frame_time {
                renderer.gpu.window.request_redraw();
                target.set_control_flow(ControlFlow::WaitUntil(Instant::now() + frame_time));
            }
        } else {
            renderer.gpu.window.request_redraw();
            target.set_control_flow(ControlFlow::Poll);
        }

        loop_store.elapsed = Instant::now().duration_since(loop_store.frame_start);
    })?;

    Ok(())
}
