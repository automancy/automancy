#![windows_subsystem = "windows"]
use std::{
    env,
    fmt::Write,
    fs,
    fs::File,
    panic,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use automancy_lib::*;
use camera::GameCamera;
use color_eyre::config::HookBuilder;
use cosmic_text::fontdb::Source;
use game::{GameSystem, GameSystemMessage, TICK_INTERVAL};
use glam::uvec2;
use gpu::Gpu;
use input::InputHandler;
use kira::{
    manager::{AudioManager, AudioManagerSettings},
    track::{TrackBuilder, TrackHandle},
    tween::Tween,
};
use map::LoadMapOption;
use options::{GameOptions, MiscOptions};
use ractor::Actor;
use renderer::GameRenderer;
use rendering::Vertex;
use rfd::{MessageButtons, MessageDialog, MessageLevel};
use tokio::runtime::Runtime;
use ui_state::UiState;
use uuid::Uuid;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, Event, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Fullscreen, Icon, Window, WindowId},
};
use yakui::paint::{Texture, TextureFilter};

pub static LOGO: &[u8] = include_bytes!("logo.png");

/// Initialize the Resource Manager system, and loads all the resources in all namespaces.
fn load_resources(
    selected_language: &str,
    track: TrackHandle,
) -> (Arc<ResourceManager>, Vec<Vertex>, Vec<u16>) {
    let mut resource_man = ResourceManager::new(track);

    fs::read_dir(RESOURCES_PATH)
        .expect("The resources folder doesn't exist- this is very wrong")
        .flatten()
        .map(|v| v.path())
        .filter(|v| v.is_dir())
        .for_each(|dir| {
            let namespace = dir.file_name().unwrap().to_str().unwrap().trim();
            log::info!("Loading namespace {namespace}...");

            resource_man
                .load_models(&dir, namespace)
                .expect("Error loading models");

            resource_man.load_audio(&dir).expect("Error loading audio");

            resource_man
                .load_tiles(&dir, namespace)
                .expect("Error loading tiles");

            resource_man
                .load_items(&dir, namespace)
                .expect("Error loading items");

            resource_man
                .load_tags(&dir, namespace)
                .expect("Error loading tags");
            resource_man
                .load_categories(&dir, namespace)
                .expect("Error loading categories");

            resource_man
                .load_scripts(&dir, namespace)
                .expect("Error loading scripts");

            resource_man
                .load_translates(&dir, namespace, selected_language)
                .expect("Error loading translates");

            resource_man
                .load_shaders(&dir)
                .expect("Error loading shaders");

            resource_man.load_fonts(&dir).expect("Error loading fonts");

            resource_man
                .load_functions(&dir, namespace)
                .expect("Error loading functions");

            resource_man
                .load_researches(&dir, namespace)
                .expect("Error loading researches");

            log::info!("Loaded namespace {namespace}.");
        });

    resource_man
        .engine
        .definitions()
        .with_headers(true)
        .include_standard_packages(false)
        .write_to_dir("rhai")
        .unwrap();

    resource_man.compile_researches();
    resource_man.ordered_tiles();
    resource_man.ordered_items();
    resource_man.compile_categories();

    let (vertices, indices) = resource_man.compile_models();

    (Arc::new(resource_man), vertices, indices)
}

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

struct Automancy {
    state: GameState,
    window: Option<Arc<Window>>,
    fps_limit: Option<i32>,
    closed: bool,
}

impl Automancy {
    fn try_sync_options(&mut self) {
        if !self.state.options.synced {
            {
                let font = self
                    .state
                    .resource_man
                    .fonts
                    .get(
                        &self
                            .state
                            .options
                            .gui
                            .get_font(&self.state.resource_man)
                            .expect("no fonts loaded!"),
                    )
                    .or_else(|| {
                        self.state
                            .options
                            .gui
                            .set_font(&self.state.resource_man, None);

                        self.state.resource_man.fonts.values().next()
                    })
                    .expect("no fonts loaded! at all! put one in there!");

                self.state.gui.as_mut().unwrap().set_font(
                    SYMBOLS_FONT_KEY,
                    &font.name,
                    Source::Binary(font.data.clone()),
                );
            }

            self.state
                .audio_man
                .main_track()
                .set_volume(self.state.options.audio.sfx_volume, Tween::default());

            self.state
                .renderer
                .as_mut()
                .unwrap()
                .gpu
                .set_vsync(self.state.options.graphics.fps_limit == 0);

            self.fps_limit = Some(self.state.options.graphics.fps_limit);

            if self.state.options.graphics.fullscreen {
                self.state
                    .renderer
                    .as_ref()
                    .unwrap()
                    .gpu
                    .window
                    .set_fullscreen(Some(Fullscreen::Borderless(None)));
            } else {
                self.state
                    .renderer
                    .as_ref()
                    .unwrap()
                    .gpu
                    .window
                    .set_fullscreen(None);
            }

            self.state.options.synced = true;

            log::info!("Synced options!");
        }
    }
}

impl ApplicationHandler for Automancy {
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.closed = true;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Creating window...");
        let icon = get_icon();

        let window_attributes = Window::default_attributes()
            .with_title("automancy")
            .with_window_icon(Some(icon))
            .with_min_inner_size(PhysicalSize::new(200, 200));

        self.window = Some(Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to open window"),
        ));
        log::info!("Window created.");

        let gpu = self.state.tokio.block_on(Gpu::new(
            self.window.as_ref().unwrap().clone(),
            self.state.options.graphics.fps_limit == 0,
        ));

        log::info!("Setting up rendering...");
        let (shared_resources, render_resources, global_resources) = gpu::init_gpu_resources(
            &gpu.device,
            &gpu.config,
            &self.state.resource_man,
            self.state.vertices_init.take().unwrap(),
            self.state.indices_init.take().unwrap(),
        );
        let global_resources = Arc::new(global_resources);
        let renderer = GameRenderer::new(
            gpu,
            shared_resources,
            render_resources,
            global_resources.clone(),
        );
        log::info!("Render setup.");

        log::info!("Setting up gui...");
        let mut gui = GameGui::new(
            &renderer.gpu.device,
            &renderer.gpu.queue,
            &renderer.gpu.window,
        );

        gui.window.set_automatic_scale_factor(false);
        gui.yak.set_scale_factor(
            (renderer.gpu.window.scale_factor() * self.state.options.graphics.ui_scale.to_f64())
                as f32,
        );

        gui.fonts.insert(
            SYMBOLS_FONT_KEY.to_string(),
            cosmic_text::fontdb::Source::Binary(Arc::from(&SYMBOLS_FONT)),
        );
        for (name, font) in self.state.resource_man.fonts.iter() {
            gui.fonts.insert(
                name.clone(),
                cosmic_text::fontdb::Source::Binary(font.data.clone()),
            );
        }
        log::info!("Gui setup.");

        let logo = image::load_from_memory(LOGO).unwrap();
        let mut logo = Texture::new(
            yakui::paint::TextureFormat::Rgba8Srgb,
            uvec2(logo.width(), logo.height()),
            logo.into_bytes(),
        );
        logo.mag_filter = TextureFilter::Linear;
        logo.min_filter = TextureFilter::Linear;
        let logo = gui.yak.add_texture(logo);

        self.state.logo = Some(logo);
        self.state.gui = Some(gui);
        self.state.renderer = Some(renderer);

        self.try_sync_options();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if !self.closed {
            let consumed = {
                let gui = self.state.gui.as_mut().unwrap();
                gui.window.handle_window_event(&mut gui.yak, &event)
            };

            if consumed {
                return;
            }

            match event::on_event(
                &mut self.state,
                event_loop,
                Event::WindowEvent { window_id, event },
            ) {
                Ok(closed) => {
                    self.closed = closed;
                }
                Err(e) => {
                    log::warn!("Window event error: {e}");
                }
            }

            self.try_sync_options();
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if !self.closed {
            match event::on_event(
                &mut self.state,
                event_loop,
                Event::DeviceEvent { device_id, event },
            ) {
                Ok(closed) => {
                    self.closed = closed;
                }
                Err(e) => {
                    log::warn!("Device event error: {e}");
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let fps_limit = self.fps_limit.unwrap_or(0);

        if fps_limit != 0 {
            let frame_time = if fps_limit >= 250 {
                Duration::ZERO
            } else {
                Duration::from_secs_f64(1.0 / fps_limit as f64)
            };

            let elapsed = self.state.loop_store.frame_start.unwrap().elapsed();
            if elapsed < frame_time {
                let time_left = frame_time - elapsed;

                event_loop.set_control_flow(ControlFlow::wait_duration(time_left));
                return;
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Poll);
        }

        self.window.as_ref().unwrap().request_redraw();
    }
}

fn main() -> anyhow::Result<()> {
    env::set_var("RUST_BACKTRACE", "full");

    {
        let filter = "info,wgpu_core::device::resource=warn";

        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(filter)).init();

        #[cfg(debug_assertions)]
        {
            use tracing_subscriber::{EnvFilter, prelude::__tracing_subscriber_SubscriberExt};
            use tracing_tracy::DefaultConfig;

            tracing::subscriber::set_global_default({
                tracing_subscriber::registry()
                    .with(tracing_tracy::TracyLayer::new(DefaultConfig::default()))
                    .with(EnvFilter::from_env(filter))
            })?;
        }
    }

    {
        let eyre = HookBuilder::blank()
            .capture_span_trace_by_default(true)
            .display_env_section(false);

        let (panic_hook, eyre_hook) = eyre.into_hooks();

        eyre_hook.install()?;

        panic::set_hook(Box::new(move |info| {
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

        log::info!("Initializing audio backend...");
        let mut audio_man = AudioManager::new(AudioManagerSettings::default())?;
        log::info!("Audio backend initialized");

        log::info!("Loading resources...");
        let track = audio_man.add_sub_track({
            let builder = TrackBuilder::new();

            builder
        })?;

        let misc_options = MiscOptions::load();

        let (resource_man, vertices, indices) = load_resources(&misc_options.language, track);
        RESOURCE_MAN.write().unwrap().replace(resource_man.clone());
        log::info!("Loaded resources.");

        let options = GameOptions::load(&resource_man);
        let input_handler = InputHandler::new(&options);

        let mut loop_store = EventLoopStorage::default();
        let camera = GameCamera::new((1.0, 1.0)); // dummy value

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

        let start_instant = Instant::now();
        ui_game_object::init_custom_paint_state(start_instant);
        loop_store.frame_start = Some(start_instant);

        GameState {
            ui_state: UiState::default(),
            options,
            misc_options,
            resource_man,
            input_handler,
            loop_store,
            tokio,
            game,
            camera,
            audio_man,
            start_instant,

            gui: None,
            renderer: None,
            screenshotting: false,

            logo: Default::default(),
            input_hints: Default::default(),
            puzzle_state: Default::default(),

            game_handle: Some(game_handle),

            vertices_init: Some(vertices),
            indices_init: Some(indices),
        }
    };

    // load the main menu
    game_load_map_inner(&mut state, LoadMapOption::MainMenu);

    let mut automancy = Automancy {
        state,
        window: None,
        fps_limit: None,
        closed: false,
    };

    event_loop.run_app(&mut automancy)?;

    Ok(())
}
