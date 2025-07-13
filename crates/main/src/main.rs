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

use automancy_data::math::Vec2;
use automancy_game::{
    actor::{TICK_INTERVAL, game::GameActor, map::GameMapId, message::GameMsg},
    input::{camera::GameCamera, handler::InputHandler},
    persistent::options::{GameOptions, MiscOptions},
    resources::{RESOURCES_PATH, ResourceManager, global},
    state::{AutomancyGameState, event::EventLoopStorage, ui::UiState},
};
use automancy_lib::{render::AutomancyRendering, *};
use automancy_rendering::renderer::AutomancyRenderState;
use color_eyre::config::HookBuilder;
use kira::{
    AudioManager, AudioManagerSettings, Tween,
    track::{TrackBuilder, TrackHandle},
};
use ractor::Actor;
use rfd::{MessageButtons, MessageDialog, MessageLevel};
use tokio::runtime::Runtime;
use uuid::Uuid;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, Event, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub static LOGO: &[u8] = include_bytes!("logo.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomancyGameLoadResult {
    Loaded,
    LoadedMainMenu,
    Failed,
}

fn game_load_map(state: &mut AutomancyGameState, id: GameMapId) -> AutomancyGameLoadResult {
    let success = match state.tokio.block_on(
        state
            .game
            .call(|reply| GameMsg::LoadMap(id.clone(), reply), None),
    ) {
        Ok(v) => v.unwrap(),
        Err(_) => false,
    };

    if success {
        state.loop_store.map_info = state
            .tokio
            .block_on(state.game.call(GameMsg::GetMapIdAndData, None))
            .unwrap()
            .unwrap();

        AutomancyGameLoadResult::Loaded
    } else if id != GameMapId::MainMenu {
        game_load_map(state, GameMapId::MainMenu)
    } else {
        log::warn!("Loading empty map as fallback.");
        game_load_map(state, GameMapId::Empty)
    }
}

/// Initialize the ResourceManager, and loads all the resources in all namespaces.
fn load_resources(lang: &str, track: TrackHandle) -> Arc<ResourceManager> {
    let mut resource_man = ResourceManager::new(track);

    fs::read_dir(RESOURCES_PATH)
        .expect("the resources folder needs to exist and be readable")
        .flatten()
        .map(|v| v.path())
        .filter(|v| v.is_dir())
        .for_each(|dir| {
            let namespace = dir.file_name().unwrap().to_str().unwrap().trim();
            log::info!("Loading namespace {namespace}...");

            resource_man.load_models(&dir, namespace).unwrap();
            resource_man.load_audio(&dir).unwrap();
            resource_man.load_tiles(&dir, namespace).unwrap();
            resource_man.load_items(&dir, namespace).unwrap();
            resource_man.load_tags(&dir, namespace).unwrap();
            resource_man.load_categories(&dir, namespace).unwrap();
            resource_man.load_recipes(&dir, namespace).unwrap();
            resource_man.load_translates(&dir, namespace, lang).unwrap();
            resource_man.load_shaders(&dir).unwrap();
            resource_man.load_fonts(&dir).unwrap();
            resource_man.load_scripts(&dir, namespace).unwrap();
            resource_man.load_researches(&dir, namespace).unwrap();

            log::info!("Loaded namespace {namespace}.");
        });

    resource_man
        .engine
        .definitions()
        .with_headers(true)
        .include_standard_packages(false)
        .write_to_dir("rhai")
        .unwrap();

    resource_man.ordered_tiles();
    resource_man.ordered_items();
    resource_man.compile_researches();
    resource_man.compile_categories();

    Arc::new(resource_man)
}

fn get_window_icon() -> winit::window::Icon {
    let image = image::load_from_memory(LOGO).unwrap().to_rgba8();
    let width = image.width();
    let height = image.height();

    let samples = image.into_flat_samples().samples;
    winit::window::Icon::from_rgba(samples, width, height).unwrap()
}

fn write_crash_msg<P: AsRef<Path>>(buffer: &mut impl Write, file_path: P) -> std::fmt::Result {
    writeln!(buffer, "Well, this is embarrassing.")?;
    writeln!(
        buffer,
        "automancy! had a problem and crashed. To help us diagnose the problem, you can send us a crash report."
    )?;

    writeln!(buffer)?;

    writeln!(
        buffer,
        "The game has generated a report at file://{} for more information,",
        file_path.as_ref().display(),
    )?;
    writeln!(
        buffer,
        "submit an issue on the Git repository or tag us in the Discord server, and include the report as an attachment:"
    )?;
    writeln!(buffer, "- Git: https://github.com/automancy/automancy")?;
    writeln!(buffer, "- Discord: https://discord.gg/ee9XebxNaa")?;
    writeln!(buffer, "Alternatively, send an Email to these addresses:")?;
    writeln!(buffer, "- Madeline Sparkles (madeline@mouse.lgbt)")?;

    writeln!(buffer)?;

    writeln!(
        buffer,
        "We take privacy seriously, and do not perform any kinds of automated error collection. In order to improve the game, we rely on people to submit reports."
    )?;
    writeln!(buffer, "Thank you kindly!")?;

    Ok(())
}

struct Automancy {
    game_state: AutomancyGameState,
    render_state: AutomancyRenderState,
    render: Option<AutomancyRendering>,
    closed: bool,
}

impl Automancy {
    fn try_sync_options(&mut self) {
        if !self.game_state.options.synced {
            {
                let font = self
                    .game_state
                    .resource_man
                    .fonts
                    .get(
                        &self
                            .game_state
                            .options
                            .gui
                            .get_font(&self.game_state.resource_man)
                            .expect("the specified font should be loaded"),
                    )
                    .or_else(|| {
                        self.game_state
                            .options
                            .gui
                            .set_font(&self.game_state.resource_man, None);

                        self.game_state.resource_man.fonts.values().next()
                    })
                    .expect("no fonts loaded at all, at least one font needs to be present");

                /*
                self.game_state.gui.as_mut().unwrap().set_font(
                    SYMBOLS_FONT_KEY,
                    &font.name,
                    Source::Binary(font.data.clone()),
                );
                 */
            }

            self.game_state
                .audio_man
                .main_track()
                .set_volume(self.game_state.options.audio.sfx_volume, Tween::default());

            self.render
                .as_mut()
                .unwrap()
                .res
                .set_vsync(self.game_state.options.graphics.fps_limit == 0);

            if self.game_state.options.graphics.fullscreen {
                self.render
                    .as_ref()
                    .unwrap()
                    .res
                    .window
                    .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            } else {
                self.render
                    .as_ref()
                    .unwrap()
                    .res
                    .window
                    .set_fullscreen(None);
            }

            self.game_state.options.synced = true;

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
        let icon = get_window_icon();
        let window_attributes = Window::default_attributes()
            .with_title("automancy")
            .with_window_icon(Some(icon))
            .with_min_inner_size(PhysicalSize::new(200, 200));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        log::info!("Window created.");

        log::info!("Setting up rendering...");
        self.render = Some(self.game_state.tokio.block_on(AutomancyRendering::new(
            &self.game_state.resource_man,
            &self.render_state,
            window.clone(),
        )));
        log::info!("Render setup.");

        log::info!("Setting up gui...");
        /* TODO reimpl
        let mut gui = GameGui::new(
            &renderer.gpu.device,
            &renderer.gpu.queue,
            &renderer.gpu.window,
        );
        gui.window.set_automatic_scale_factor(false);
        gui.yak.set_scale_factor(
            (renderer.gpu.window.scale_factor()
                * self.game_state.options.graphics.ui_scale.to_f64()) as f32,
        );

        gui.fonts.insert(
            SYMBOLS_FONT_KEY.to_string(),
            cosmic_text::fontdb::Source::Binary(Arc::from(&SYMBOLS_FONT)),
        );
        for (name, font) in self.game_state.resource_man.fonts.iter() {
            gui.fonts.insert(
                name.clone(),
                cosmic_text::fontdb::Source::Binary(font.data.clone()),
            );
        }
         */
        log::info!("Gui setup.");

        let logo = image::load_from_memory(LOGO).unwrap();
        let mut logo = yakui::paint::Texture::new(
            yakui::paint::TextureFormat::Rgba8Srgb,
            yakui::UVec2::new(logo.width(), logo.height()),
            logo.into_bytes(),
        );
        logo.mag_filter = yakui::paint::TextureFilter::Linear;
        logo.min_filter = yakui::paint::TextureFilter::Linear;
        // TODO reimpl let logo = gui.yak.add_texture(logo);

        self.try_sync_options();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if !self.closed {
            /*  TODO reimpl
            let consumed = {
                let gui = self.game_state.gui.as_mut().unwrap();
                gui.window.handle_window_event(&mut gui.yak, &event)
            };

            if consumed {
                return;
            }
            */

            match integration::on_event(
                event_loop,
                &mut self.game_state,
                &mut self.render_state,
                self.render.as_mut().unwrap(),
                Event::WindowEvent { window_id, event },
            ) {
                Ok(_) => {}
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
            match integration::on_event(
                event_loop,
                &mut self.game_state,
                &mut self.render_state,
                self.render.as_mut().unwrap(),
                Event::DeviceEvent { device_id, event },
            ) {
                Ok(()) => {}
                Err(e) => {
                    log::warn!("Device event error: {e}");
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let fps_limit = self.game_state.options.graphics.fps_limit;

        if fps_limit != 0 {
            let frame_time = if fps_limit >= 250 {
                Duration::ZERO
            } else {
                Duration::from_secs_f64(1.0 / fps_limit as f64)
            };

            let elapsed = self.game_state.loop_store.frame_start.unwrap().elapsed();
            if elapsed < frame_time {
                let time_left = frame_time - elapsed;

                event_loop.set_control_flow(ControlFlow::wait_duration(time_left));
                return;
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Poll);
        }

        self.render.as_ref().unwrap().res.window.request_redraw();
    }
}

fn main() -> anyhow::Result<()> {
    // SAFETY: we are on the main thread
    unsafe {
        env::set_var("RUST_BACKTRACE", "full");
    }

    {
        let filter = "info,wgpu_core::device::resource=warn";

        let mut builder =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(filter));
        if let Ok(file) = env::var("LOG_FILE") {
            let file = Box::new(File::create(file).expect("log file needs to created"));

            builder.target(env_logger::Target::Pipe(file));
        }
        builder.init();

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

            {
                let mut message = String::new();
                _ = write_crash_msg(&mut message, &file_path);

                eprintln!("\n{}", message);

                _ = MessageDialog::new()
                    .set_level(MessageLevel::Error)
                    .set_buttons(MessageButtons::Ok)
                    .set_title("automancy crash dialog")
                    .set_description(message)
                    .show();
            }
        }));
    }

    let mut game_state = {
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

        let resource_man = load_resources(&misc_options.language, track);
        global::set_resource_man(resource_man.clone());
        log::info!("Loaded resources.");

        let options = GameOptions::load(&resource_man);
        let input_handler = InputHandler::new(&options);

        let mut loop_store = EventLoopStorage::default();
        let camera = GameCamera::new(
            Vec2::one(), // dummy value
        );

        log::info!("Creating game...");
        let (game, game_handle) = tokio.block_on(Actor::spawn(
            Some("game".to_string()),
            GameActor {
                resource_man: resource_man.clone(),
            },
            (),
        ))?;
        {
            let game = game.clone();
            tokio.spawn(async move {
                game.send_interval(TICK_INTERVAL, || GameMsg::Tick);
            });
        }
        log::info!("Game created.");

        let start_instant = Instant::now();
        // TODO reimpl ui_game_object::init_custom_paint_state(start_instant);
        loop_store.frame_start = Some(start_instant);

        AutomancyGameState {
            resource_man,
            loop_store,
            ui_state: UiState::default(),
            input_handler,
            audio_man,
            camera,

            tokio,
            game,
            game_handle: Some(game_handle),

            options,
            misc_options,

            start_instant,

            input_hints: Default::default(),
            puzzle_state: Default::default(),
        }
    };

    log::info!("Loading rendering resources...");
    let mut render_state = AutomancyRenderState::default();
    render_state.model_man.load_models(&game_state.resource_man);
    log::info!("Loaded rendering resources.");

    // load the main menu
    game_load_map(&mut game_state, GameMapId::MainMenu);

    let mut automancy = Automancy {
        game_state,
        render_state,
        render: None,
        closed: false,
    };

    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut automancy)?;

    Ok(())
}
