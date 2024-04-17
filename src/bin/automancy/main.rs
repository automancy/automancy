#![windows_subsystem = "windows"]

use std::panic::PanicInfo;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{collections::BTreeMap, fmt::Write};
use std::{env, fs, panic};
use std::{fs::File, mem};

use color_eyre::config::HookBuilder;
use env_logger::Env;
use num::Zero;
use once_cell::sync::Lazy;
use ractor::Actor;
use rfd::{MessageButtons, MessageDialog, MessageLevel};
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Fullscreen, Icon, WindowBuilder};

use automancy::event::{on_event, EventLoopStorage};
use automancy::gpu::{init_gpu_resources, Gpu};
use automancy::gui::GuiState;
use automancy::input::InputHandler;
use automancy::map::MAIN_MENU;
use automancy::options::Options;
use automancy::renderer::Renderer;
use automancy::{camera::Camera, gui::Gui};
use automancy::{
    game::{load_map, GameSystem, GameSystemMessage, TICK_INTERVAL},
    gui::init_custom_paint_state,
};
use automancy::{GameState, LOGO};
use automancy_defs::glam::uvec2;
use automancy_defs::rendering::Vertex;
use automancy_defs::{log, window};
use automancy_resources::kira::track::{TrackBuilder, TrackHandle};
use automancy_resources::kira::tween::Tween;
use automancy_resources::{
    kira::manager::{AudioManager, AudioManagerSettings},
    types::font::Font,
};
use automancy_resources::{ResourceManager, RESOURCES_PATH, RESOURCE_MAN};
use yakui::paint::Texture;

/// Initialize the Resource Manager system, and loads all the resources in all namespaces.
fn load_resources(
    track: TrackHandle,
) -> (
    Arc<ResourceManager>,
    Vec<Vertex>,
    Vec<u16>,
    BTreeMap<String, Font>,
) {
    let mut resource_man = ResourceManager::new(track);

    fs::read_dir(RESOURCES_PATH)
        .expect("The resources folder doesn't exist- this is very wrong")
        .flatten()
        .map(|v| v.path())
        .for_each(|dir| {
            let namespace = dir.file_name().unwrap().to_str().unwrap();
            log::info!("Loading namespace {namespace}...");

            resource_man
                .load_models(&dir)
                .expect("Error loading models");
            resource_man.load_audio(&dir).expect("Error loading audio");
            resource_man.load_tiles(&dir).expect("Error loading tiles");
            resource_man.load_items(&dir).expect("Error loading items");
            resource_man.load_tags(&dir).expect("Error loading tags");
            resource_man
                .load_categories(&dir)
                .expect("Error loading categories");
            resource_man
                .load_scripts(&dir)
                .expect("Error loading scripts");
            resource_man
                .load_translates(&dir)
                .expect("Error loading translates");
            resource_man
                .load_shaders(&dir)
                .expect("Error loading shaders");
            resource_man.load_fonts(&dir).expect("Error loading fonts");
            resource_man
                .load_functions(&dir)
                .expect("Error loading functions");
            resource_man
                .load_researches(&dir)
                .expect("Error loading researches");

            log::info!("Loaded namespace {namespace}.");
        });

    resource_man.compile_researches();
    resource_man.ordered_tiles();
    resource_man.ordered_items();
    resource_man.ordered_categories();

    let (vertices, indices) = resource_man.compile_models();
    let fonts = mem::take(&mut resource_man.fonts);

    (Arc::new(resource_man), vertices, indices, fonts)
}

static SYMBOLS_FONT: &[u8] = include_bytes!("../../assets/SymbolsNerdFontMono-Regular.ttf");
static SYMBOLS_FONT_KEY: &str = "SYMBOLS_FONT";

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

fn main() -> anyhow::Result<()> {
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

    let event_loop = EventLoop::new()?;

    let mut state = {
        let tokio = Runtime::new().unwrap();

        log::info!("Creating window...");
        let icon = get_icon();

        let window = WindowBuilder::new()
            .with_title("automancy")
            .with_window_icon(Some(icon))
            .with_min_inner_size(PhysicalSize::new(200, 200))
            .build(&event_loop)
            .expect("Failed to open window");
        log::info!("Window created.");

        let options = Options::load()?;
        let input_handler = InputHandler::new(&options);

        let loop_store = EventLoopStorage::new();
        let camera = Camera::new(window::window_size_double(&window));

        log::info!("Initializing audio backend...");
        let mut audio_man = AudioManager::new(AudioManagerSettings::default())?;
        log::info!("Audio backend initialized");

        log::info!("Loading resources...");
        let track = audio_man.add_sub_track({
            let builder = TrackBuilder::new();

            builder
        })?;

        let (resource_man, vertices, indices, fonts) = load_resources(track);
        RESOURCE_MAN.write().unwrap().replace(resource_man.clone());
        log::info!("Loaded resources.");

        log::info!("Creating game...");
        let (game, game_handle) = tokio.block_on(Actor::spawn(
            Some("game".to_string()),
            GameSystem {
                resource_man: resource_man.clone(),
            },
            (),
        ))?;
        {
            let game = game.clone();
            tokio.spawn(async move {
                game.send_interval(TICK_INTERVAL, || GameSystemMessage::Tick);
            });
        }
        log::info!("Game created.");

        let gpu = tokio.block_on(Gpu::new(Arc::new(window), options.graphics.fps_limit == 0));

        log::info!("Setting up rendering...");
        let (shared_resources, render_resources, global_buffers, gui_resources) =
            init_gpu_resources(
                &gpu.device,
                &gpu.queue,
                &gpu.config,
                &resource_man,
                vertices,
                indices,
            );
        let global_buffers = Arc::new(global_buffers);
        let renderer = Renderer::new(
            gpu,
            shared_resources,
            render_resources,
            global_buffers.clone(),
            gui_resources,
            &options,
        );
        log::info!("Render setup.");

        log::info!("Setting up gui...");
        let mut gui = Gui::new(
            &renderer.gpu.device,
            &renderer.gpu.queue,
            &renderer.gpu.window,
        );

        gui.font_names = fonts
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect();

        gui.fonts.insert(
            SYMBOLS_FONT_KEY.to_string(),
            Lazy::new(Box::new(|| {
                yakui::font::Font::from_bytes(SYMBOLS_FONT, yakui::font::FontSettings::default())
                    .unwrap()
            })),
        );
        for (name, font) in fonts.into_iter() {
            gui.fonts.insert(
                name,
                Lazy::new(Box::new(move || {
                    yakui::font::Font::from_bytes(font.data, yakui::font::FontSettings::default())
                        .unwrap()
                })),
            );
        }
        gui.set_font(SYMBOLS_FONT_KEY, &options.gui.font);
        log::info!("Gui setup.");

        let start_instant = Instant::now();
        init_custom_paint_state(start_instant);

        let logo = image::load_from_memory(LOGO).unwrap();
        let logo = gui.yak.add_texture(Texture::new(
            yakui::paint::TextureFormat::Rgba8Srgb,
            uvec2(logo.width(), logo.height()),
            logo.into_bytes(),
        ));

        GameState {
            gui_state: GuiState::new(),
            input_handler,
            options,
            resource_man,
            camera,
            loop_store,
            tokio,
            game,
            gui,
            renderer,
            game_handle: Some(game_handle),
            start_instant,
            audio_man,
            puzzle_state: Default::default(),
            logo,
        }
    };

    let mut closed = false;

    // load the main menu
    state
        .tokio
        .block_on(load_map(
            &state.game,
            &mut state.loop_store,
            MAIN_MENU.to_string(),
        ))
        .unwrap();

    event_loop.run(move |event, target| {
        if closed {
            return;
        }

        match on_event(&mut state, target, event) {
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

        if !state.options.synced {
            state
                .gui
                .set_font(SYMBOLS_FONT_KEY, &state.options.gui.font);

            state
                .audio_man
                .main_track()
                .set_volume(state.options.audio.sfx_volume, Tween::default())
                .unwrap();

            state
                .renderer
                .gpu
                .set_vsync(state.options.graphics.fps_limit == 0);

            if state.options.graphics.fullscreen {
                state
                    .renderer
                    .gpu
                    .window
                    .set_fullscreen(Some(Fullscreen::Borderless(None)));
            } else {
                state.renderer.gpu.window.set_fullscreen(None);
            }

            state.options.synced = true;
        }

        if !state.renderer.fps_limit.is_zero() {
            let frame_time;

            if state.options.graphics.fps_limit >= 250 {
                frame_time = Duration::ZERO;
            } else {
                frame_time = Duration::from_secs_f64(1.0 / state.renderer.fps_limit as f64);
            }

            if state.loop_store.frame_start.elapsed() > frame_time {
                state.renderer.gpu.window.request_redraw();
                target.set_control_flow(ControlFlow::WaitUntil(Instant::now() + frame_time));
            }
        } else {
            state.renderer.gpu.window.request_redraw();
            target.set_control_flow(ControlFlow::Poll);
        }

        let new_elapsed = Instant::now().duration_since(state.loop_store.frame_start);
        state.loop_store.elapsed = new_elapsed;
    })?;

    Ok(())
}
