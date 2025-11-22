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
    actor::{
        game::{GameActor, TICK_INTERVAL},
        message::GameMsg,
    },
    input::{InputHandler, camera::GameCamera},
    persistent::{
        map::GameMapId,
        options::{GameOptions, MiscOptions},
    },
    state::{AutomancyGameState, GameDataStorage, ui::UiState},
};
use automancy_lib::integration::{self, WindowExt};
use automancy_rendering::{
    gpu,
    gpu::RenderResources,
    renderer::{AutomancyRenderState, AutomancyRendering},
};
use kira::{AudioManager, AudioManagerSettings, Tween, track::TrackBuilder};
use ractor::Actor;
use tokio::runtime::Runtime;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, Event, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

mod panic;
mod util;

#[cfg(debug_assertions)]
mod debug;
#[cfg(debug_assertions)]
use debug::*;

fn prepare_screenshot(res: &RenderResources, encoder: &mut wgpu::CommandEncoder, surface_size: wgpu::Extent3d) -> wgpu::Buffer {
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
        &res.present_res.present_texture.create_view(&wgpu::TextureViewDescriptor::default()),
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
    res: &RenderResources,
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

    let padded_data = slice.get_mapped_range().to_vec();
    let mut data = Vec::new();

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
    render_state: AutomancyRenderState,
    render: Option<AutomancyRendering>,

    clipboard: arboard::Clipboard,

    closed: bool,

    #[cfg(debug_assertions)]
    debug_console_state: Option<DebugConsoleState>,
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
                        self.game_state.options.gui.set_font(&self.game_state.resource_man, None);

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
                self.window
                    .as_deref()
                    .unwrap()
                    .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            } else {
                self.window.as_deref().unwrap().set_fullscreen(None);
            }

            self.game_state.options.synced = true;

            log::info!("Synced options!");
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
}

impl ApplicationHandler for Automancy {
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.closed = true;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Creating window...");
        let icon = util::get_window_icon();
        let window_attributes = Window::default_attributes()
            .with_title("automancy")
            .with_window_icon(Some(icon))
            .with_min_inner_size(PhysicalSize::new(200, 200));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window.set_ime_allowed(true);
        self.window = Some(window);
        log::info!("Window created.");

        log::info!("Setting up rendering...");
        self.render = Some(self.game_state.tokio.block_on(AutomancyRendering::new(
            &self.game_state.resource_man,
            &self.render_state,
            self.window.clone().unwrap(),
        )));
        self.render.as_mut().unwrap().res.resize(self.window.as_deref().unwrap().size_uvec2());
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

        let logo = image::load_from_memory(util::GAME_LOGO).unwrap();
        let mut logo = yakui::paint::Texture::new(
            yakui::paint::TextureFormat::Rgba8Srgb,
            yakui::UVec2::new(logo.width(), logo.height()),
            logo.into_bytes(),
        );
        logo.mag_filter = yakui::paint::TextureFilter::Linear;
        logo.min_filter = yakui::paint::TextureFilter::Linear;
        // TODO reimpl let logo = gui.yak.add_texture(logo);

        #[cfg(debug_assertions)]
        {
            let mut debug = DebugConsoleState::new();
            let window = self.window.as_deref().unwrap();
            debug.resize(
                window.size_uvec2(),
                window.scale_factor() as f32,
                &self.render.as_ref().unwrap().res.device,
            );

            self.debug_console_state = Some(debug);
        }
        self.try_sync_options();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        self.try_sync_options();

        if event == WindowEvent::CloseRequested {
            log::info!("Window close event received! Shutting down the game now.");
            self.shutdown_game(event_loop);
            return;
        }

        if !self.closed {
            let window = self.window.as_deref().unwrap();
            let render = self.render.as_mut().unwrap();

            /*  TODO reimpl
            let consumed = {
                let gui = self.game_state.gui.as_mut().unwrap();
                gui.window.handle_window_event(&mut gui.yak, &event)
            };

            if consumed {
                return;
            }
            */
            match event {
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    /* TODO reimpl
                    state.gui.as_mut().unwrap().yak.set_scale_factor(
                        (*scale_factor * state.options.graphics.ui_scale.to_f64()) as f32,
                    );
                    */

                    #[cfg(debug_assertions)]
                    self.debug_console_state
                        .as_mut()
                        .unwrap()
                        .resize(window.size_uvec2(), scale_factor as f32, &render.res.device);
                    return;
                }
                WindowEvent::Resized(size) => {
                    render.res.resize(UVec2::new(size.width, size.height));

                    #[cfg(debug_assertions)]
                    self.debug_console_state
                        .as_mut()
                        .unwrap()
                        .resize(window.size_uvec2(), window.scale_factor() as f32, &render.res.device);
                }
                _ => (),
            }

            #[cfg(debug_assertions)]
            {
                use winit::{
                    keyboard::{Key, NamedKey},
                    platform::modifier_supplement::KeyEventExtModifierSupplement,
                };

                let debug = self.debug_console_state.as_mut().unwrap();

                if let WindowEvent::KeyboardInput { event, .. } = &event
                    && event.state.is_pressed()
                    && event.key_without_modifiers() == Key::Named(NamedKey::F5)
                {
                    debug.active = !debug.active;
                    debug.resize(window.size_uvec2(), window.scale_factor() as f32, &render.res.device);

                    log::info!("Debug console state set to: {}", if debug.active { "Enabled" } else { "Disabled" });
                }

                if debug.handle_event(&mut self.game_state, &event, &mut self.clipboard) {
                    return;
                }
            }

            if event == WindowEvent::RedrawRequested {
                let window_size = window.inner_size();
                if window_size.width == 0 || window_size.height == 0 {
                    return;
                }

                match render.res.surface.get_current_texture() {
                    Ok(surface) => {
                        let surface_size = surface.texture.size();
                        if surface_size.width != window_size.width || surface_size.height != window_size.height {
                            return;
                        }

                        let mut encoder = render.res.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                        integration::render(window, &surface, render, &mut self.game_state, &mut self.render_state, &mut encoder);

                        let surface_view = surface.texture.create_view(&wgpu::TextureViewDescriptor {
                            label: Some("Surface Texture"),
                            usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
                            ..Default::default()
                        });

                        #[allow(unused_mut)]
                        let mut has_overlay_texture = false;

                        #[cfg(debug_assertions)]
                        {
                            let debug = self.debug_console_state.as_mut().unwrap();
                            if debug.active {
                                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("Debug Console Render Pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &surface_view,
                                        depth_slice: None,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                });

                                debug.draw(
                                    &render.res.device,
                                    &render.res.queue,
                                    &render.res.config,
                                    &render.res.global_res,
                                    &render
                                        .res
                                        .present_res
                                        .present_texture
                                        .create_view(&wgpu::TextureViewDescriptor::default()),
                                    render_pass,
                                );
                                has_overlay_texture = true;
                            }
                        }

                        if !has_overlay_texture {
                            wgpu::util::TextureBlitter::new(&render.res.device, surface.texture.format()).copy(
                                &render.res.device,
                                &mut encoder,
                                &render
                                    .res
                                    .present_res
                                    .present_texture
                                    .create_view(&wgpu::TextureViewDescriptor::default()),
                                &surface_view,
                            );
                        }

                        let screenshot_buffer = if render.screenshotting {
                            render.screenshotting = false;
                            Some(prepare_screenshot(&render.res, &mut encoder, surface_size))
                        } else {
                            None
                        };

                        render.res.queue.submit([encoder.finish()]);
                        window.pre_present_notify();
                        surface.present();

                        if let Some(buffer) = screenshot_buffer {
                            copy_screenshot_to_clipboard(&render.res, buffer, surface_size, &mut self.clipboard);
                        }
                    }
                    Err(wgpu::SurfaceError::Lost) => {
                        log::warn!("GPU surface is lost! Attempting to recreate the swapchain.");

                        render.res.resize(window.size_uvec2());
                    }
                    Err(wgpu::SurfaceError::Outdated) => {
                        log::warn!("GPU surface is outdated! Attempting to recreate the swapchain.");

                        render.res.resize(window.size_uvec2());
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("GPU ran out of memory! Shutting down the game now.");

                        self.shutdown_game(event_loop);
                    }
                    Err(e) => log::error!("GPU surface error: {e:?}"),
                }

                return;
            }

            match integration::handle_winit_event(window, render, &mut self.game_state, Event::WindowEvent { window_id, event }) {
                Ok(_) => {}
                Err(e) => {
                    log::warn!("Window event error: {e}");
                }
            }
        } else {
            log::info!("Window event received after game closure: {event:?}");
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        if !self.closed {
            let window = self.window.as_deref().unwrap();
            let render = self.render.as_mut().unwrap();

            match integration::handle_winit_event(window, render, &mut self.game_state, Event::DeviceEvent { device_id, event }) {
                Ok(_) => {}
                Err(e) => {
                    log::warn!("Device event error: {e}");
                }
            }
        } else {
            log::warn!("Device event received after game closure: {event:?}");
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.closed {
            let fps_limit = self.game_state.options.graphics.fps_limit;

            if fps_limit != 0 {
                let frame_time = if fps_limit >= 250 {
                    Duration::ZERO
                } else {
                    Duration::from_secs_f64(1.0 / fps_limit as f64)
                };

                let elapsed = self.render.as_ref().unwrap().frame_start.elapsed();
                if elapsed < frame_time {
                    let time_left = frame_time - elapsed;

                    event_loop.set_control_flow(ControlFlow::wait_duration(time_left));
                    return;
                }
            } else {
                event_loop.set_control_flow(ControlFlow::Poll);
            }

            self.window.as_deref().unwrap().request_redraw();
        }
    }
}

fn main() -> anyhow::Result<()> {
    // SAFETY: we are on the main thread
    unsafe {
        env::set_var("RUST_BACKTRACE", "full");
    }

    {
        let filter = "info,wgpu_core::device::resource=warn";

        let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(filter));
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

    panic::install_panic_hook()?;

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

        let resource_man = integration::load_resources(&misc_options.language, track);
        log::info!("Loaded resources.");

        let options = GameOptions::load(&resource_man);

        log::info!("Creating game...");
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
                game_handle.send_interval(TICK_INTERVAL, || GameMsg::Tick);
            });
        }
        log::info!("Game created.");

        // TODO reimpl ui_game_object::init_custom_paint_state(start_instant);

        AutomancyGameState {
            resource_man,
            audio_man,
            tokio,

            ui_state: UiState::default(),

            input_handler: InputHandler::new(&options),
            camera: GameCamera::new(
                Vec2::one(), // dummy value
            ),

            options,
            misc_options,

            game_data: GameDataStorage::default(),

            game_handle,
            game_join_handle: Some(game_join_handle),

            start_instant: Instant::now(),
        }
    };

    log::info!("Loading rendering resources...");
    let mut render_state = AutomancyRenderState::default();
    render_state.model_man.load_models(&game_state.resource_man);
    log::info!("Loaded rendering resources.");

    // load the main menu
    // TODO debug map is temporary
    game_state.load_map(GameMapId::MainMenu);

    let mut automancy = Automancy {
        window: None,
        game_state,
        render_state,
        render: None,

        clipboard: arboard::Clipboard::new().unwrap(),

        closed: false,

        #[cfg(debug_assertions)]
        debug_console_state: None,
    };

    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut automancy)?;

    Ok(())
}
