#![windows_subsystem = "windows"]

use std::{
    borrow::Cow,
    env,
    fs::File,
    sync::Arc,
    time::{Duration, Instant},
};

use automancy_data::math::{UVec2, Vec2};
use automancy_game::{
    actor,
    actor::{game_entity::GameActor, message::GameMsg},
    input::{InputHandler, camera::GameCamera},
    persistent::{
        map::GameMapId,
        options::{GameOptions, MiscOptions},
    },
    state::{AutomancyGameState, GameDataStorage},
};
use automancy_rendering::{gpu, renderer};
use automancy_ui::AutomancyUiContext;
use ractor::Actor;
use tokio::runtime::Runtime;
use wgpu::CurrentSurfaceTexture;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, Event, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

mod integration;
mod panic;
mod util;

#[cfg(debug_assertions)]
mod debug;
#[cfg(debug_assertions)]
use debug::*;

fn prepare_screenshot(res: &gpu::RenderResources, surface_size: wgpu::Extent3d, encoder: &mut wgpu::CommandEncoder) -> wgpu::Buffer {
    let screenshot_pixel_data_size = gpu::util::copy_texture_size(surface_size, gpu::SCREENSHOT_FORMAT, gpu::SCREENSHOT_PIXEL_SIZE);

    let buffer_size = gpu::util::pixel_data_buffer_size(screenshot_pixel_data_size);

    let texture = res.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Screenshot Texture"),
        size: surface_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: gpu::SCREENSHOT_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let buffer = res.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Screenshot Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    wgpu::util::TextureBlitter::new(&res.device, gpu::SCREENSHOT_FORMAT).copy(
        &res.device,
        encoder,
        &res.main_game_res.render_textures.output_texture,
        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
    );

    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(screenshot_pixel_data_size.width),
                rows_per_image: Some(screenshot_pixel_data_size.height),
            },
        },
        surface_size,
    );

    buffer
}

fn copy_screenshot_to_clipboard(
    res: &gpu::RenderResources,
    screenshot_buffer: wgpu::Buffer,
    surface_size: wgpu::Extent3d,
    clipboard: &mut arboard::Clipboard,
) {
    let screenshot_pixel_data_size = gpu::util::copy_texture_size(surface_size, gpu::SCREENSHOT_FORMAT, gpu::SCREENSHOT_PIXEL_SIZE);

    let slice = screenshot_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, move |result| {
        result.unwrap();
    });
    res.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let padded_data = slice.get_mapped_range().unwrap();
    let mut data = Vec::with_capacity(padded_data.len());

    let padded_width = (screenshot_pixel_data_size.width) as usize;
    let unpadded_width = (surface_size.width * gpu::SCREENSHOT_PIXEL_SIZE) as usize;

    #[cfg(debug_assertions)]
    let mut count = 0u32;

    for chunk in padded_data.chunks(padded_width) {
        data.extend(&chunk[..unpadded_width]);

        #[cfg(debug_assertions)]
        {
            count += 1;
        }
    }

    #[cfg(debug_assertions)]
    debug_assert_eq!(count, surface_size.height);

    clipboard
        .set_image(arboard::ImageData {
            width: surface_size.width as usize,
            height: surface_size.height as usize,
            bytes: Cow::Owned(data),
        })
        .unwrap();
}

struct Automancy {
    window: Option<Arc<Window>>,
    game_state: AutomancyGameState,
    game_data: GameDataStorage,
    render_state: renderer::AutomancyRenderState,
    render: Option<renderer::AutomancyRendering>,
    gui: Option<automancy_ui::AutomancyGui>,

    clipboard: arboard::Clipboard,

    closing: bool,
    closed: bool,

    #[cfg(debug_assertions)]
    debug_console_state: Option<DebugConsoleState>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl Automancy {
    fn sync_options(&mut self) {
        self.gui
            .as_mut()
            .unwrap()
            .yak
            .set_scale_factor(self.window.as_deref().unwrap().scale_factor() as f32 * self.game_state.options.graphics.ui_scale.to_f32());

        if self.game_state.options.gui.system_fonts() {
            self.gui.as_mut().unwrap().use_system_fonts();
        } else {
            self.gui.as_mut().unwrap().use_built_in_fonts(&self.game_state.resource_man);
        }

        if self.game_state.options.gui.system_fonts_preferences() {
            self.gui.as_mut().unwrap().set_font_family_system();
        } else {
            let font = self
                .game_state
                .options
                .gui
                .font()
                .filter(|&font| self.gui.as_ref().unwrap().font_families.iter().any(|v| v == font))
                .map(str::to_string)
                .unwrap_or_else(|| {
                    log::warn!(
                        "In applying settings: Invalid font '{}'!",
                        self.game_state.options.gui.font().unwrap_or("<none>")
                    );
                    log::warn!("Did the font get removed? Resetting font option.");

                    self.game_state.options.gui.set_font(None);
                    let first_font = self
                        .gui
                        .as_ref()
                        .unwrap()
                        .font_families
                        .first()
                        .map(ToString::to_string)
                        .expect("at least one font should be loaded");
                    self.game_state.options.gui.set_font(Some(first_font.clone()));

                    first_font
                });

            self.gui.as_mut().unwrap().set_font_family(font);
        }

        #[cfg(not(miri))]
        // TODO uh, this lol
        self.game_state
            .audio_man
            .main_track()
            .set_volume(self.game_state.options.audio.sfx_volume, kira::Tween::default());

        let render = self.render.as_mut().unwrap();

        render.res.set_vsync(self.game_state.options.graphics.fps_limit() == 0);
        render.res.set_antialiasing_type(self.game_state.options.graphics.antialiasing_type);

        if self.game_state.options.graphics.fullscreen {
            self.window
                .as_deref()
                .unwrap()
                .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        } else {
            self.window.as_deref().unwrap().set_fullscreen(None);
        }
    }

    fn sync_misc_options(&mut self) {
        // unimplemented
    }

    fn try_sync_options(&mut self) {
        if !self.game_state.misc_options.synced {
            log::info!("Syncing misc options...");
            self.sync_misc_options();

            log::info!("Saving misc options after syncing...");
            self.game_state.misc_options.save().unwrap();
            self.game_state.misc_options.synced = true;

            log::info!("Synced misc options!");
        }

        if !self.game_state.options.synced {
            log::info!("Syncing game options...");
            self.sync_options();

            log::info!("Saving game options after syncing...");
            self.game_state.options.save().unwrap();
            self.game_state.options.synced = true;

            log::info!("Synced game options!");
        }
    }

    fn shutdown_game(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.exit();

        {
            let game_handle = self.game_state.game_handle.clone();
            let game_join_handle = self.game_state.game_join_handle.take().expect("game handle needs to be set");

            self.game_state.tokio.block_on(async {
                game_handle
                    .call(GameMsg::SaveAndUnload, None)
                    .await
                    .unwrap()
                    .unwrap()
                    .expect("the game needs to save the map on exit");
                game_handle.stop(Some("game closed".to_string()));

                game_join_handle.await.unwrap();
            });
        }

        log::info!("Shut down gracefully.");
        self.closed = true;
    }

    fn resize(&mut self, window_size: UVec2, scale_factor: f32) {
        let render = self.render.as_mut().unwrap();
        render.res.resize(window_size);

        self.gui
            .as_mut()
            .unwrap()
            .yak
            .set_scale_factor(scale_factor * self.game_state.options.graphics.ui_scale.to_f32());

        #[cfg(debug_assertions)]
        self.debug_console_state
            .as_mut()
            .unwrap()
            .resize(window_size, scale_factor, &render.res.device);
    }

    fn render(&mut self, surface_texture: wgpu::SurfaceTexture) {
        let window = self.window.as_deref().unwrap();
        let render = self.render.as_mut().unwrap();
        let gui = self.gui.as_mut().unwrap();

        let viewport_size = render.res.viewport_size_u32();
        let surface_size = surface_texture.texture.size();

        if surface_size.width != viewport_size.x || surface_size.height != viewport_size.y {
            render.res.resize(viewport_size);
            return;
        }

        {
            #[cfg(feature = "profile")]
            profiling::scope!("update_render_info");

            //game_state.camera.set_fov(/* TODO */);
            self.game_state.camera.set_viewport_size(render.res.viewport_size_f32());

            self.game_data.refresh_async_data(&mut self.game_state);
            render.frame_time = render.frame_start.elapsed();
            render.frame_start = Instant::now();

            self.game_state.camera.update(render.frame_time.as_secs_f32());
            self.render_state.update(&self.game_state);

            render.game_renderer.update_tile_hints(&mut self.render_state);
        }

        {
            #[cfg(feature = "profile")]
            profiling::scope!("layout_ui");

            automancy_ui::layout(&mut AutomancyUiContext {
                gui,
                render,
                render_state: &mut self.render_state,
                game_state: &mut self.game_state,
                game_data: &mut self.game_data,
                winit_window: window,
                closing: &mut self.closing,
            });
        }

        let surface_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Surface Texture"),
            usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
            ..Default::default()
        });

        let mut screenshot_buffer = None;
        {
            {
                #[cfg(feature = "profile")]
                profiling::scope!("render_ui");

                let mut encoder = render.res.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("UI Render Encoder"),
                });
                automancy_ui::render(gui, render, &mut self.render_state, &self.game_state.resource_man, &mut encoder);

                render.res.queue.submit([encoder.finish()]);
            }

            {
                #[cfg(feature = "profile")]
                profiling::scope!("render_game");

                let mut encoder = render.res.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Game Render Encoder"),
                });

                {
                    render.game_renderer.render(
                        &mut render.res,
                        &mut self.render_state,
                        &self.game_state,
                        render.frame_start,
                        &mut encoder,
                    );
                }

                if render.screenshotting {
                    screenshot_buffer = Some(prepare_screenshot(&render.res, surface_size, &mut encoder));
                    render.screenshotting = false;
                }

                render.res.queue.submit([encoder.finish()]);
            }

            {
                #[cfg(feature = "profile")]
                profiling::scope!("present");

                let mut encoder = render.res.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Present Encoder"),
                });

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Present Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &surface_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });

                    render_pass.set_pipeline(&render.res.present_res.compose_pipeline.render_pipeline);
                    render_pass.set_bind_group(0, &render.res.present_res.compose_pipeline.textures_bind_group, &[]);
                    render_pass.set_bind_group(1, &render.res.present_res.compose_pipeline.uniform_bind_group, &[]);
                    render_pass.draw(0..3, 0..1);
                }

                #[cfg(debug_assertions)]
                {
                    let debug = self.debug_console_state.as_mut().unwrap();
                    if debug.active {
                        #[cfg(feature = "profile")]
                        profiling::scope!("render_debug");

                        debug.draw(&render.res.device, &render.res.queue, &mut encoder, &surface_view);
                    }
                }

                render.res.queue.submit([encoder.finish()]);
            }
        }

        window.pre_present_notify();
        render.res.queue.present(surface_texture);

        render.frame_count = render.frame_count.wrapping_add(1);

        #[cfg(feature = "profile-with-tracy")]
        tracing_tracy::client::frame_mark();

        if let Some(buffer) = screenshot_buffer {
            copy_screenshot_to_clipboard(&render.res, buffer, surface_size, &mut self.clipboard);
        }
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl ApplicationHandler for Automancy {
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.closing = false;
        self.closed = true;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Creating the window...");
        let icon = util::get_window_icon();
        let window_attributes = Window::default_attributes()
            .with_title("automancy")
            .with_window_icon(Some(icon))
            .with_min_inner_size(PhysicalSize::new(200, 200));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window.set_ime_allowed(true);
        log::info!("Window created! We have finally invented the window, how amazing.");

        log::info!("Setting up the rendering...");
        let render = self.game_state.tokio.block_on(renderer::AutomancyRendering::new(
            &self.game_state.resource_man,
            &self.render_state,
            window.clone(),
            event_loop.owned_display_handle(),
        ));
        log::info!("Render's all setup!");

        log::info!("Setting up the UI...");
        let gui = {
            let logo = image::load_from_memory(util::GAME_LOGO).unwrap();
            let mut logo = yakui::paint::Texture::new(
                yakui::paint::TextureFormat::Rgba8Srgb,
                yakui::UVec2::new(logo.width(), logo.height()),
                logo.into_bytes(),
            );
            logo.mag_filter = yakui::paint::TextureFilter::Linear;
            logo.min_filter = yakui::paint::TextureFilter::Linear;

            automancy_ui::AutomancyGui::new(&render.res.device, &render.res.queue, &window, logo)
        };
        log::info!("UI's all setup!");

        #[cfg(debug_assertions)]
        {
            log::info!("This build has debugging enabled! Setting up the debug console...");
            self.debug_console_state = Some(DebugConsoleState::new());
            log::info!("The debug console is real! Press {DEBUG_CONSOLE_KEY:?} to toggle the console <3.");
        }

        self.gui = Some(gui);
        self.render = Some(render);

        let window_size = window.inner_size();
        let window_size = UVec2::new(window_size.width, window_size.height);
        let scale_factor = window.scale_factor();

        log::info!("Initial window size: {window_size} (scale: {scale_factor})");
        self.resize(window_size, scale_factor as f32);

        self.window = Some(window);

        log::info!("Doing an initial sync for the options.");
        self.try_sync_options();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if self.closed {
            log::info!("Window event received after game closure: {event:?}");
            return;
        }

        self.try_sync_options();

        match event {
            WindowEvent::ScaleFactorChanged {
                scale_factor, ..
            } => {
                let window_size = self.window.as_deref().unwrap().inner_size();
                let window_size = UVec2::new(window_size.width, window_size.height);

                self.resize(window_size, scale_factor as f32);
            },
            WindowEvent::Resized(window_size) => {
                let scale_factor = self.window.as_deref().unwrap().scale_factor();
                let window_size = UVec2::new(window_size.width, window_size.height);

                self.resize(window_size, scale_factor as f32);
            },
            WindowEvent::CloseRequested => {
                log::info!("Window close event received! Shutting down the game now.");
                self.shutdown_game(event_loop);
                return;
            },
            WindowEvent::RedrawRequested => {
                let render = self.render.as_mut().unwrap();

                let viewport_size = render.res.viewport_size_u32();
                if viewport_size.x == 0 || viewport_size.y == 0 {
                    return;
                }

                match render.res.surface.get_current_texture() {
                    CurrentSurfaceTexture::Success(surface_texture) => {
                        self.render(surface_texture);
                    },
                    CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                        self.render(surface_texture);

                        log::warn!("GPU surface is suboptimal! Attempting to recreate the pipeline.");
                        let render = self.render.as_mut().unwrap();
                        render.res.recreate();
                    },
                    CurrentSurfaceTexture::Outdated => {
                        log::warn!("GPU surface is outdated! Attempting to recreate the pipeline.");
                        render.res.recreate();
                    },
                    CurrentSurfaceTexture::Lost => {
                        log::warn!("GPU surface is lost! Attempting to recreate the pipeline.");
                        panic!("unimplemented")
                    },
                    CurrentSurfaceTexture::Timeout => {
                        log::warn!("Render timeout! The current frame is skipped.");
                    },
                    CurrentSurfaceTexture::Occluded => {},
                    CurrentSurfaceTexture::Validation => {
                        // ? do we need to log anything?
                    },
                }

                let render = self.render.as_mut().unwrap();
                let is_vsync = render.res.is_vsync();
                if !is_vsync {
                    let fps_limit = self.game_state.options.graphics.fps_limit();

                    let frame_time = if fps_limit == 0 || fps_limit >= 250 {
                        Duration::ZERO
                    } else {
                        Duration::from_secs_f64(1.0 / fps_limit as f64)
                    };

                    event_loop.set_control_flow(ControlFlow::WaitUntil(render.frame_start + frame_time));
                } else {
                    event_loop.set_control_flow(ControlFlow::Poll);
                }

                return;
            },
            _ => {},
        }

        let window = self.window.as_deref().unwrap();
        let render = self.render.as_mut().unwrap();

        #[cfg(debug_assertions)]
        {
            let debug = self.debug_console_state.as_mut().unwrap();

            if let WindowEvent::KeyboardInput {
                event, ..
            } = &event
            {
                use winit::{keyboard::Key, platform::modifier_supplement::KeyEventExtModifierSupplement};

                if event.state.is_pressed() && event.key_without_modifiers() == Key::Named(DEBUG_CONSOLE_KEY) {
                    debug.resize(render.res.viewport_size_u32(), window.scale_factor() as f32, &render.res.device);
                    debug.active = !debug.active;

                    log::info!("Debug console state set to: {}", if debug.active { "Enabled" } else { "Disabled" });
                }
            }

            if debug.active && debug.handle_event(&mut self.game_state, &mut self.game_data, &event, &mut self.clipboard) {
                return;
            }
        }

        let gui = self.gui.as_mut().unwrap();
        if gui.yakui_winit.handle_window_event(&mut gui.yak, &event, window) {
            return;
        }

        match integration::handle_winit_event(
            &mut AutomancyUiContext {
                gui,
                render,
                render_state: &mut self.render_state,
                game_state: &mut self.game_state,
                game_data: &mut self.game_data,
                winit_window: window,
                closing: &mut self.closing,
            },
            Event::WindowEvent {
                window_id,
                event,
            },
        ) {
            Ok(_) => {},
            Err(e) => {
                log::warn!("Window event error: {e}");
            },
        }

        if self.closing {
            log::info!("Closing game...");
            self.closing = false;
            self.shutdown_game(event_loop);
        }
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        if self.closed {
            log::warn!("Device event received after game closure: {event:?}");
            return;
        }

        let window = self.window.as_deref().unwrap();
        let render = self.render.as_mut().unwrap();
        let gui = self.gui.as_mut().unwrap();

        match integration::handle_winit_event(
            &mut AutomancyUiContext {
                gui,
                render,
                render_state: &mut self.render_state,
                game_state: &mut self.game_state,
                game_data: &mut self.game_data,
                winit_window: window,
                closing: &mut self.closing,
            },
            Event::DeviceEvent {
                device_id,
                event,
            },
        ) {
            Ok(_) => {},
            Err(e) => {
                log::warn!("Device event error: {e}");
            },
        }

        if self.closing {
            log::info!("Closing game...");
            self.closing = false;
            self.shutdown_game(event_loop);
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        match cause {
            StartCause::ResumeTimeReached {
                ..
            }
            | StartCause::Poll => {
                self.window.as_deref().unwrap().request_redraw();
            },
            _ => {},
        }
    }
}

fn main() -> anyhow::Result<()> {
    // SAFETY: we are on the main thread
    unsafe {
        env::set_var("RUST_BACKTRACE", "full");
    }

    {
        let mut builder = env_logger::Builder::new();
        builder
            .filter(Some("wgpu_core::device::resource"), log::LevelFilter::Warn)
            .filter(Some("yakui_core"), log::LevelFilter::Info)
            .filter(Some("cosmic_text"), log::LevelFilter::Info)
            .filter_level(log::LevelFilter::Info)
            .parse_default_env();

        if let Ok(file) = env::var("LOG_FILE") {
            let file = Box::new(File::create(file).expect("log file needs to created"));

            builder.target(env_logger::Target::Pipe(file));
        }

        builder.init();

        log::info!("Logger initialized!");
    }

    #[cfg(feature = "profile")]
    {
        use tracing_subscriber::{EnvFilter, prelude::*};

        log::info!("Profiling feature is enabled!");
        let builder = tracing_subscriber::registry();
        let builder = builder.with(EnvFilter::new("trace"));

        #[cfg(feature = "profile-with-tracy")]
        {
            log::info!("Tracy feature is enabled!");

            tracing_tracy::client::register_demangler!();
            tracing_tracy::client::Client::start();

            builder.with(tracing_tracy::TracyLayer::default()).init();

            log::info!("Tracy feature initialized!");
        }

        #[cfg(feature = "profile-with-tracing")]
        {
            builder.init();
        }

        log::info!("Profiling feature initialized!");
    }

    panic::install_panic_hook()?;

    let mut game_state = {
        let tokio = Runtime::new().unwrap();

        #[cfg(not(miri))]
        let audio_man = {
            log::info!("Initializing audio backend...");
            let audio_man = kira::AudioManager::new(kira::AudioManagerSettings::default())?;
            log::info!("Audio backend initialized.");

            audio_man
        };
        #[cfg(miri)]
        let audio_man = automancy_game::miri::StubbedAudioManager;

        log::info!("Parsing some miscellaneous options...");
        let misc_options = MiscOptions::load();
        log::info!("Miscellaneous options loaded.");

        log::info!("Searching for resources and loading them...");
        let resource_man = integration::load_resources(&misc_options.language);
        log::info!("Resources loaded.");

        log::info!("Parsing game options...");
        let options = GameOptions::load(&resource_man);
        log::info!("Game options loaded.");

        log::info!("Creating the actor system for the game...");
        let (game_handle, game_join_handle) = tokio.block_on(Actor::spawn(
            Some("game".to_string()),
            GameActor {
                resource_man: resource_man.clone(),
            },
            (),
        ))?;
        {
            let game_handle = game_handle.clone();
            tokio.spawn(async move {
                game_handle.send_interval(actor::game_entity::TICK_INTERVAL, || GameMsg::Tick);
            });
        }
        log::info!("Actor system created.");

        AutomancyGameState {
            resource_man,
            audio_man,
            tokio,

            input_handler: InputHandler::new(&options),
            camera: GameCamera::new(
                Vec2::one(), // dummy value
            ),
            config_open_at: None,

            options,
            misc_options,

            game_handle,
            game_join_handle: Some(game_join_handle),

            game_start: Instant::now(),
        }
    };

    log::info!("Loading rendering resources...");
    let mut render_state = renderer::AutomancyRenderState::default();
    render_state.model_man.load_models(&game_state.resource_man);
    log::info!("Loaded rendering resources.");

    log::info!("Loading main menu...");
    let mut game_data = GameDataStorage::default();
    game_state.load_map(&mut game_data, GameMapId::MainMenu);

    let mut automancy = Automancy {
        window: None,
        game_state,
        game_data,
        render_state,
        render: None,
        gui: None,

        clipboard: arboard::Clipboard::new().unwrap(),

        closing: false,
        closed: false,

        #[cfg(debug_assertions)]
        debug_console_state: None,
    };

    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut automancy)?;

    Ok(())
}
