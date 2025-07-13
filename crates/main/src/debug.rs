use core::{
    cmp::{max, min},
    str::FromStr,
};
use std::collections::VecDeque;

use automancy_data::math::UVec2;
use automancy_game::{persistent::map::GameMapId, resources::types::font, state::AutomancyGameState};
use automancy_rendering::gpu::{ComposePipeline, ComposePipelineArgs, GlobalResources};
use cosmic_text::Edit;
use fuzzy_matcher::FuzzyMatcher;
use hashbrown::HashMap;
use strum::VariantNames;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, Modifiers, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, NamedKey},
};

static CONSOLE_LICENSE: &str = include_str!("assets/Iosevka-LICENSE.md");
static CONSOLE_FONT: &[u8] = include_bytes!("assets/IosevkaFixed-Regular.ttf");

const COMMAND_BUFFER_FONT_SIZE: f32 = 20.0;
const COMMAND_BUFFER_LINE_HEIGHT: f32 = 22.0;
const COMMAND_HISTORY_LEN: usize = 64;

#[derive(Debug, strum::EnumDiscriminants)]
#[strum_discriminants(name(DebugCommandType))]
#[strum_discriminants(derive(strum::EnumString, strum::VariantNames, strum::IntoStaticStr, strum::Display))]
#[strum_discriminants(strum(serialize_all = "snake_case", ascii_case_insensitive))]
enum DebugCommand {
    Help,
    Clear,
    Beep,
    Boop,
    FontLicense,
    Set(DebugState),
}

#[derive(Debug, strum::EnumDiscriminants)]
#[strum_discriminants(name(DebugStateType))]
#[strum_discriminants(derive(strum::EnumString, strum::VariantNames, strum::IntoStaticStr, strum::Display))]
#[strum_discriminants(strum(serialize_all = "snake_case", ascii_case_insensitive))]
enum DebugState {
    LoadDebugMap(bool),
}

struct CommandHandler {
    debug_map_loaded: bool,
    matcher: fuzzy_matcher::skim::SkimMatcherV2,
}

#[allow(clippy::derivable_impls)]
impl Default for CommandHandler {
    fn default() -> Self {
        Self {
            debug_map_loaded: false,
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default().use_cache(true).ignore_case(),
        }
    }
}

#[derive(Debug)]
enum CommandParseError {
    UnknownCommand {
        command: String,
    },
    InvalidArgument {
        loc: &'static str,
        valid_args: &'static [&'static str],
    },
    ExpectingBoolAt {
        command: Vec<&'static str>,
        loc: &'static str,
        args_list: &'static [&'static str],
    },
}

impl CommandHandler {
    fn fmt_arg_list(args_list: &[&str]) -> String {
        args_list.iter().map(|v| format!("<{v}>")).collect::<Vec<_>>().join(" ")
    }

    fn parse_tokens(command: &str) -> VecDeque<&str> {
        let mut tokens = command.split_whitespace().collect::<VecDeque<_>>();
        let last_char = command.chars().last().unwrap_or_default();

        if last_char.is_whitespace() {
            tokens.push_back("");
        }

        tokens
    }

    fn parse_command(mut tokens: VecDeque<&str>) -> Result<DebugCommand, CommandParseError> {
        if tokens.is_empty() {
            return Err(CommandParseError::UnknownCommand { command: String::new() });
        }

        let command = tokens.pop_front();
        if let Some(ty) = command.and_then(|s| DebugCommandType::from_str(s).ok()) {
            match ty {
                DebugCommandType::Help => Ok(DebugCommand::Help),
                DebugCommandType::Clear => Ok(DebugCommand::Clear),
                DebugCommandType::Beep => Ok(DebugCommand::Beep),
                DebugCommandType::Boop => Ok(DebugCommand::Boop),
                DebugCommandType::FontLicense => Ok(DebugCommand::FontLicense),
                DebugCommandType::Set => {
                    if let Some(ty) = tokens.pop_front().and_then(|s| DebugStateType::from_str(s).ok()) {
                        match ty {
                            DebugStateType::LoadDebugMap => {
                                const ARGS_LIST: &[&str] = &["v"];

                                if let Some(load_debug_map) = tokens.pop_front().and_then(|s| bool::from_str(s).ok()) {
                                    Ok(DebugCommand::Set(DebugState::LoadDebugMap(load_debug_map)))
                                } else {
                                    Err(CommandParseError::ExpectingBoolAt {
                                        command: vec![DebugCommandType::Set.into(), DebugStateType::LoadDebugMap.into()],
                                        loc: ARGS_LIST[0],
                                        args_list: ARGS_LIST,
                                    })
                                }
                            }
                        }
                    } else {
                        Err(CommandParseError::InvalidArgument {
                            loc: "state_name",
                            valid_args: DebugStateType::VARIANTS,
                        })
                    }
                }
            }
        } else {
            Err(CommandParseError::UnknownCommand {
                command: command.unwrap_or_default().to_string(),
            })
        }
    }

    pub fn get_completion(&self, buffer: &cosmic_text::Buffer) -> Vec<String> {
        let command = buffer.lines.iter().map(|v| v.text()).collect::<Vec<_>>().join("\n");

        let mut tokens = Self::parse_tokens(&command);

        match Self::parse_command(tokens.clone()) {
            Ok(_) => Vec::new(),
            Err(err) => {
                let mut args = Vec::new();
                let last_token = tokens.pop_back().unwrap_or_default();

                match err {
                    CommandParseError::UnknownCommand { .. } => {
                        if (command.is_empty() && last_token.is_empty()) || command == last_token {
                            args = DebugCommandType::VARIANTS.to_vec();
                        }
                    }
                    CommandParseError::InvalidArgument { valid_args, .. } => {
                        args = valid_args.to_vec();
                    }
                    CommandParseError::ExpectingBoolAt { .. } => {
                        args = vec!["true", "false"];
                    }
                }

                if args.is_empty() {
                    return Vec::new();
                }

                let mut results = Vec::new();

                for arg in args {
                    if last_token.is_empty() {
                        results.push((0, arg.to_string()));
                        continue;
                    }

                    if !(arg.starts_with(last_token)) {
                        continue;
                    }

                    let score = self.matcher.fuzzy_match(arg, last_token);
                    let score = max(score.unwrap_or(0), 0) as usize;
                    if score > (arg.len() / 2) {
                        results.push((score, arg.to_string()));
                    }
                }
                results.sort_by(|a, b| a.1.cmp(&b.1));
                results.sort_by_key(|v| v.0);

                results.into_iter().map(|v| v.1).collect()
            }
        }
    }

    pub fn handle_command(&mut self, game_state: &mut AutomancyGameState, buffer: &str, command: &str) -> Result<String, String> {
        let response;
        match Self::parse_command(Self::parse_tokens(command)) {
            Ok(command) => match command {
                DebugCommand::Help => response = format!("Available commands: [{}]", DebugCommandType::VARIANTS.join(", ")),
                DebugCommand::Clear => return Ok(String::new()),
                DebugCommand::Beep => response = "Boop!".to_string(),
                DebugCommand::Boop => response = "Beep!".to_string(),
                DebugCommand::FontLicense => response = CONSOLE_LICENSE.to_string(),
                DebugCommand::Set(DebugState::LoadDebugMap(load_debug_map)) => {
                    #[allow(clippy::collapsible_else_if)]
                    if load_debug_map {
                        if !self.debug_map_loaded {
                            response = "Loading debug map...".to_string();
                            game_state.load_map(GameMapId::Debug);
                        } else {
                            response = "Debug map already loaded!".to_string();
                        }
                    } else {
                        if self.debug_map_loaded {
                            response = "Loading main menu map...".to_string();
                            game_state.load_map(GameMapId::MainMenu);
                        } else {
                            response = "Debug map wasn't loaded!".to_string();
                        }
                    }

                    self.debug_map_loaded = load_debug_map;
                }
            },
            Err(CommandParseError::InvalidArgument { loc, valid_args }) => {
                return Err(format!("invalid argument at <{loc}>, valid args: [{}]", valid_args.join(", ")));
            }
            Err(CommandParseError::ExpectingBoolAt { command, loc, args_list }) => {
                return Err(format!(
                    "expecting a 'bool' at <{}>, usage: {} {}",
                    loc,
                    command.join(" "),
                    Self::fmt_arg_list(args_list)
                ));
            }
            Err(CommandParseError::UnknownCommand { command }) => {
                return Err(format!(
                    "unknown command '{}', available commands: [{}]",
                    command,
                    DebugCommandType::VARIANTS.join(", "),
                ));
            }
        }

        Ok(format!("{response}\n{buffer}"))
    }
}

type GlyphCache = HashMap<(cosmic_text::Color, cosmic_text::CacheKey), (i32, i32, Option<tiny_skia::Pixmap>)>;

/// Handles and renders a debug console, triggered by pressing F5. This should only be available in debugging context.
pub struct DebugConsoleState {
    pub active: bool,

    pub redraw: bool,
    window_size: UVec2,
    texture: Option<wgpu::Texture>,

    mouse_x: f32,
    mouse_y: f32,
    mouse_left: ElementState,
    modifiers: Modifiers,

    swash_cache: cosmic_text::SwashCache,
    font_system: cosmic_text::FontSystem,
    metrics: cosmic_text::Metrics,
    command_editor: cosmic_text::Editor<'static>,
    console_buffer: cosmic_text::Buffer,
    glyph_cache: GlyphCache,

    command_history: VecDeque<String>,
    command_history_pos: Option<usize>,
    command_state: CommandHandler,
    completion_pos: Option<usize>,
    completions: Vec<String>,
}

impl DebugConsoleState {
    pub fn new() -> Self {
        let swash_cache = cosmic_text::SwashCache::new();

        let mut font_system = cosmic_text::FontSystem::new();
        font_system.db_mut().load_font_data(CONSOLE_FONT.into());
        {
            let name = font::get_font_family_name(ttf_parser::Face::parse(CONSOLE_FONT, 0).unwrap().names()).unwrap();
            font_system.db_mut().set_sans_serif_family(&name);
            font_system.db_mut().set_serif_family(&name);
            font_system.db_mut().set_monospace_family(&name);
            font_system.db_mut().set_cursive_family(&name);
            font_system.db_mut().set_fantasy_family(&name);
        }

        let metrics = cosmic_text::Metrics {
            font_size: COMMAND_BUFFER_FONT_SIZE,
            line_height: COMMAND_BUFFER_LINE_HEIGHT,
        };

        let mut command_editor = cosmic_text::Editor::new(cosmic_text::Buffer::new(&mut font_system, metrics));
        command_editor.with_buffer_mut(|buffer| buffer.set_wrap(&mut font_system, cosmic_text::Wrap::None));

        let console_buffer = cosmic_text::Buffer::new(&mut font_system, metrics);

        let mut this = Self {
            active: false,

            redraw: true,
            // resize later
            window_size: UVec2::zero(),
            texture: None,

            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_left: ElementState::Released,
            modifiers: Modifiers::default(),

            swash_cache,
            font_system,
            metrics,
            command_editor,
            console_buffer,
            glyph_cache: HashMap::default(),

            command_history: VecDeque::with_capacity(COMMAND_HISTORY_LEN),
            command_history_pos: None,
            command_state: CommandHandler::default(),
            completion_pos: None,
            completions: Vec::new(),
        };

        this.refresh_completion();

        this
    }
}

impl DebugConsoleState {
    fn push_command_history(&mut self, command: String) {
        self.command_history_pos = None;

        if self.command_history.back() != Some(&command) {
            if self.command_history.len() == COMMAND_HISTORY_LEN {
                self.command_history.pop_front();
            }
            self.command_history.push_back(command);
        }
    }

    fn refresh_completion(&mut self) {
        self.completion_pos = None;
        self.command_editor.with_buffer(|buffer| {
            self.completions = self.command_state.get_completion(buffer);
        });
    }

    fn commit_completion(&mut self) -> bool {
        if let Some(completion) = self.completion_pos.take().and_then(|i| self.completions.get(i)) {
            let mut text_to_insert = String::new();

            self.command_editor.with_buffer_mut(|buffer| {
                let command = cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, "");
                let mut tokens = CommandHandler::parse_tokens(&command);
                cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, &command);

                text_to_insert = completion.strip_prefix(tokens.pop_back().unwrap_or_default()).unwrap().to_string();
            });

            self.command_editor.insert_string(&text_to_insert, None);
            self.refresh_completion();

            return true;
        }

        false
    }

    pub fn resize(&mut self, window_size: UVec2, scale_factor: f32, device: &wgpu::Device) {
        if self.window_size != window_size || self.texture.is_none() {
            self.command_editor.with_buffer_mut(|buffer| {
                buffer.set_size(&mut self.font_system, Some(window_size.x as f32), Some(COMMAND_BUFFER_LINE_HEIGHT));

                buffer.set_metrics(&mut self.font_system, self.metrics.scale(scale_factor));
            });
            self.console_buffer.set_size(
                &mut self.font_system,
                Some(window_size.x as f32),
                Some((window_size.y as f32 - COMMAND_BUFFER_LINE_HEIGHT).max(0.0)),
            );
            self.console_buffer.set_metrics(&mut self.font_system, self.metrics.scale(scale_factor));

            self.texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Debug Console Texture"),
                size: wgpu::Extent3d {
                    width: window_size.x,
                    height: window_size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }));
            self.redraw = true;
        }

        self.window_size = window_size;
    }

    pub fn draw<'a>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        global_res: &GlobalResources,
        surface_texture: &wgpu::TextureView,
        mut render_pass: wgpu::RenderPass<'a>,
    ) {
        if self.redraw {
            self.redraw = false;

            // TODO since we don't wrap the command input, it overflows on the x axis
            self.command_editor.shape_as_needed(&mut self.font_system, false);
            self.console_buffer.shape_until_scroll(&mut self.font_system, false);

            {
                const PIXEL_SIZE: u32 = 4;

                let surface_size = self.window_size;
                let mut pixel_data: Vec<u8> = std::iter::repeat_n([0, 0, 0, 208], surface_size.x as usize * surface_size.y as usize)
                    .flatten()
                    .collect::<Vec<_>>();

                let mut pixmap = tiny_skia::PixmapMut::from_bytes(&mut pixel_data, surface_size.x, surface_size.y).unwrap();
                let mut paint = tiny_skia::Paint {
                    anti_alias: false,
                    ..Default::default()
                };

                cosmic_text_util::draw_text(
                    &self.console_buffer,
                    &mut self.font_system,
                    &mut self.swash_cache,
                    cosmic_text::Color::rgb(192, 192, 192),
                    &mut pixmap,
                    &mut self.glyph_cache,
                    (0, COMMAND_BUFFER_LINE_HEIGHT.round() as i32),
                );

                paint.blend_mode = tiny_skia::BlendMode::Source;
                paint.set_color_rgba8(0, 0, 0, 224);
                pixmap.fill_rect(
                    tiny_skia::Rect::from_xywh(0.0, 0.0, surface_size.x as f32, COMMAND_BUFFER_LINE_HEIGHT).unwrap(),
                    &paint,
                    tiny_skia::Transform::identity(),
                    None,
                );
                paint.blend_mode = tiny_skia::BlendMode::default();

                cosmic_text_util::draw_editor(
                    &self.command_editor,
                    cosmic_text::Color::rgb(255, 36, 36),
                    cosmic_text::Color::rgba(181, 210, 255, 127),
                    &mut paint,
                    &mut pixmap,
                );

                if let Some(completion_pos) = self.completion_pos {
                    let cursor = self.command_editor.cursor();
                    self.command_editor.with_buffer_mut(|buffer| {
                        let command = cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, "");
                        let mut tokens = CommandHandler::parse_tokens(&command);
                        let last_token = tokens.pop_back().unwrap_or_default();

                        let completion_text = &self.completions[completion_pos];
                        let completions_width = {
                            let mut width: f32 = 0.0;

                            for completion_text in &self.completions {
                                cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, completion_text);
                                width = width.max(
                                    buffer
                                        .line_layout(&mut self.font_system, cursor.line)
                                        .and_then(|layouts| layouts.iter().map(|line| line.w).max_by(|a, b| a.total_cmp(b)))
                                        .unwrap_or_default(),
                                );
                            }

                            width
                        };
                        let command_without_last_token_width = {
                            cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, command.strip_suffix(last_token).unwrap());
                            buffer
                                .line_layout(&mut self.font_system, cursor.line)
                                .and_then(|layouts| layouts.iter().map(|line| line.w).max_by(|a, b| a.total_cmp(b)))
                                .unwrap_or_default()
                        };

                        let rest_of_completions = &self.completions[min(completion_pos + 1, self.completions.len())..];

                        paint.blend_mode = tiny_skia::BlendMode::Source;
                        paint.set_color_rgba8(0, 0, 0, 240);
                        pixmap.fill_rect(
                            tiny_skia::Rect::from_xywh(
                                command_without_last_token_width,
                                0.0,
                                completions_width,
                                COMMAND_BUFFER_LINE_HEIGHT + COMMAND_BUFFER_LINE_HEIGHT * rest_of_completions.len() as f32,
                            )
                            .unwrap(),
                            &paint,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                        paint.blend_mode = tiny_skia::BlendMode::default();

                        {
                            cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, &command);
                            let command_width = buffer
                                .line_layout(&mut self.font_system, cursor.line)
                                .and_then(|layouts| layouts.iter().map(|line| line.w).max_by(|a, b| a.total_cmp(b)))
                                .unwrap_or_default();

                            cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, completion_text.strip_prefix(last_token).unwrap());
                            cosmic_text_util::draw_text(
                                buffer,
                                &mut self.font_system,
                                &mut self.swash_cache,
                                cosmic_text::Color::rgb(160, 160, 160),
                                &mut pixmap,
                                &mut self.glyph_cache,
                                (command_width.round() as i32, 0),
                            );
                        }

                        for (index, completion_text) in rest_of_completions.iter().enumerate() {
                            cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, completion_text);
                            cosmic_text_util::draw_text(
                                buffer,
                                &mut self.font_system,
                                &mut self.swash_cache,
                                cosmic_text::Color::rgb(160, 160, 160),
                                &mut pixmap,
                                &mut self.glyph_cache,
                                (
                                    command_without_last_token_width.round() as i32,
                                    COMMAND_BUFFER_LINE_HEIGHT.round() as i32 + index as i32 * COMMAND_BUFFER_LINE_HEIGHT.round() as i32,
                                ),
                            );
                        }

                        cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, &command);
                    });
                }

                self.command_editor.with_buffer(|buffer| {
                    cosmic_text_util::draw_text(
                        buffer,
                        &mut self.font_system,
                        &mut self.swash_cache,
                        cosmic_text::Color::rgb(255, 255, 255),
                        &mut pixmap,
                        &mut self.glyph_cache,
                        (0, 0),
                    );
                });

                let surface_size = wgpu::Extent3d {
                    width: surface_size.x,
                    height: surface_size.y,
                    depth_or_array_layers: 1,
                };
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: self.texture.as_ref().unwrap(),
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &pixel_data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(surface_size.width * PIXEL_SIZE),
                        rows_per_image: Some(surface_size.height),
                    },
                    surface_size,
                );
            }
        }

        if let Some(texture) = &self.texture {
            let compose_pipeline = ComposePipeline::new(
                device,
                config,
                global_res,
                ComposePipelineArgs {
                    first_texture: surface_texture,
                    first_sampler: &global_res.point_sampler,
                    second_texture: &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    second_sampler: &global_res.point_sampler,
                },
            );

            render_pass.set_pipeline(&compose_pipeline.render_pipeline);
            render_pass.set_bind_group(0, &compose_pipeline.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }

    pub fn handle_event(&mut self, game_state: &mut AutomancyGameState, event: &WindowEvent, clipboard: &mut arboard::Clipboard) -> bool {
        if self.active {
            enum Select {
                DeselectPrevAffinity,
                DeselectNextAffinity,
                CurrentCursor,
            }

            let mut execute_current_line = false;
            let mut insert_text = None;

            let mut action = None;
            let mut select = None;

            match event {
                WindowEvent::ModifiersChanged(modifiers) => self.modifiers = *modifiers,
                WindowEvent::KeyboardInput {
                    event: KeyEvent { logical_key, state, .. },
                    ..
                } if state.is_pressed() => {
                    match logical_key {
                        Key::Named(NamedKey::Escape) => {
                            if self.completion_pos.is_some() {
                                self.completion_pos = None;
                            } else {
                                self.active = false
                            }
                        }
                        Key::Named(NamedKey::Enter) => {
                            if !self.commit_completion() {
                                execute_current_line = true;
                            }
                        }
                        Key::Named(NamedKey::Tab) => {
                            if !self.completions.is_empty() {
                                if let Some(pos) = self.completion_pos {
                                    if self.modifiers.state().shift_key() {
                                        self.completion_pos = Some((pos as isize - 1).rem_euclid(self.completions.len() as isize) as usize)
                                    } else {
                                        self.completion_pos = Some((pos + 1) % self.completions.len());
                                    }
                                } else {
                                    self.completion_pos = Some(0);
                                }
                            }
                        }
                        Key::Named(NamedKey::Space) if self.completion_pos.is_some() => {
                            self.commit_completion();
                        }
                        Key::Named(NamedKey::ArrowLeft) => {
                            if ctrl_key(self.modifiers) {
                                action = Some(cosmic_text::Action::Motion(cosmic_text::Motion::PreviousWord));
                            } else {
                                action = Some(cosmic_text::Action::Motion(cosmic_text::Motion::Previous));
                            }

                            if self.modifiers.state().shift_key() {
                                select = Some(Select::CurrentCursor);
                            } else {
                                select = Some(Select::DeselectPrevAffinity);
                            }
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            if !self.commit_completion() {
                                if ctrl_key(self.modifiers) {
                                    action = Some(cosmic_text::Action::Motion(cosmic_text::Motion::NextWord));
                                } else {
                                    action = Some(cosmic_text::Action::Motion(cosmic_text::Motion::Next));
                                }

                                if self.modifiers.state().shift_key() {
                                    select = Some(Select::CurrentCursor);
                                } else {
                                    select = Some(Select::DeselectNextAffinity);
                                }
                            }
                        }
                        Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::PageUp) => {
                            if let Some(pos) = self.completion_pos {
                                self.completion_pos = Some((pos as isize - 1).rem_euclid(self.completions.len() as isize) as usize)
                            } else if self.modifiers.state().shift_key() || ctrl_key(self.modifiers) {
                                self.console_buffer.set_scroll(cosmic_text::Scroll {
                                    vertical: self.console_buffer.scroll().vertical - COMMAND_BUFFER_LINE_HEIGHT,
                                    ..self.console_buffer.scroll()
                                });
                            } else {
                                if !self.command_history.is_empty() && self.command_history_pos.is_none() {
                                    let current = cosmic_text_util::set_command_text(&mut self.command_editor, &mut self.font_system, "");

                                    self.push_command_history(current);

                                    self.command_history_pos = Some(self.command_history.len() - 1);
                                }

                                if let Some(pos) = &mut self.command_history_pos {
                                    *pos = pos.saturating_sub(1);

                                    cosmic_text_util::set_command_text(&mut self.command_editor, &mut self.font_system, &self.command_history[*pos]);
                                    self.refresh_completion();
                                }
                            }
                        }
                        Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::PageDown) => {
                            if let Some(pos) = self.completion_pos {
                                self.completion_pos = Some((pos + 1) % self.completions.len())
                            } else if self.modifiers.state().shift_key() || ctrl_key(self.modifiers) {
                                self.console_buffer.set_scroll(cosmic_text::Scroll {
                                    vertical: self.console_buffer.scroll().vertical + COMMAND_BUFFER_LINE_HEIGHT,
                                    ..self.console_buffer.scroll()
                                });
                            } else if let Some(pos) = &mut self.command_history_pos {
                                *pos = min(*pos + 1, self.command_history.len() - 1);

                                cosmic_text_util::set_command_text(&mut self.command_editor, &mut self.font_system, &self.command_history[*pos]);
                                self.refresh_completion();
                            }
                        }
                        Key::Named(NamedKey::Home) => {
                            if let Some((cursor, _)) = self.console_buffer.cursor_motion(
                                &mut self.font_system,
                                cosmic_text::Cursor {
                                    line: self.console_buffer.scroll().line,
                                    ..Default::default()
                                },
                                None,
                                cosmic_text::Motion::Home,
                            ) {
                                self.console_buffer.set_scroll(cosmic_text::Scroll {
                                    line: cursor.line,
                                    ..self.console_buffer.scroll()
                                });
                            }
                        }
                        Key::Named(NamedKey::End) => {
                            if let Some((cursor, _)) = self.console_buffer.cursor_motion(
                                &mut self.font_system,
                                cosmic_text::Cursor {
                                    line: self.console_buffer.scroll().line,
                                    ..Default::default()
                                },
                                None,
                                cosmic_text::Motion::End,
                            ) {
                                self.console_buffer.set_scroll(cosmic_text::Scroll {
                                    line: cursor.line,
                                    ..self.console_buffer.scroll()
                                });
                            }
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if self.command_editor.selection_bounds().is_none() && ctrl_key(self.modifiers) {
                                self.command_editor
                                    .set_selection(cosmic_text::Selection::Normal(self.command_editor.cursor()));
                                self.command_editor
                                    .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::PreviousWord));
                                self.command_editor.delete_selection();
                                self.refresh_completion();
                            } else {
                                action = Some(cosmic_text::Action::Backspace);
                            }
                        }
                        Key::Named(NamedKey::Delete) => {
                            action = Some(cosmic_text::Action::Delete);
                        }

                        Key::Character(key) if key.to_lowercase() == "a" && ctrl_key(self.modifiers) => {
                            self.command_editor
                                .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::BufferStart));
                            self.command_editor
                                .set_selection(cosmic_text::Selection::Normal(self.command_editor.cursor()));
                            self.command_editor
                                .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::BufferEnd));
                        }
                        Key::Character(key) if key.to_lowercase() == "x" && ctrl_key(self.modifiers) => {
                            if let Some(text) = self.command_editor.copy_selection() {
                                clipboard.set_text(&text).unwrap();
                            }
                            self.command_editor.delete_selection();
                            self.refresh_completion();
                        }
                        Key::Character(key) if key.to_lowercase() == "c" && ctrl_key(self.modifiers) => {
                            if let Some(text) = self.command_editor.copy_selection() {
                                clipboard.set_text(&text).unwrap();
                            }
                        }
                        Key::Character(key) if key.to_lowercase() == "v" && ctrl_key(self.modifiers) => {
                            if let Ok(text) = clipboard.get_text() {
                                insert_text = Some(text);
                            }
                        }

                        Key::Named(key) => {
                            insert_text = key.to_text().map(str::to_string);
                        }
                        Key::Character(text) => {
                            insert_text = Some(text.as_str().to_string());
                        }

                        _ => {}
                    }

                    self.redraw = true;
                }

                WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                    insert_text = Some(text.clone());

                    self.redraw = true;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    // Update saved mouse position for use when handling click events
                    self.mouse_x = position.x as f32;
                    self.mouse_y = position.y as f32;

                    // Implement dragging
                    if self.mouse_left.is_pressed() {
                        // Execute Drag editor action (update selection)
                        self.command_editor.action(
                            &mut self.font_system,
                            cosmic_text::Action::Drag {
                                x: position.x as i32,
                                y: position.y as i32,
                            },
                        );
                        self.redraw = true;
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if *button == MouseButton::Left {
                        if *state == ElementState::Pressed && self.mouse_left == ElementState::Released {
                            self.command_editor.set_selection(cosmic_text::Selection::None);
                            self.command_editor.action(
                                &mut self.font_system,
                                cosmic_text::Action::Click {
                                    x: self.mouse_x as i32,
                                    y: self.mouse_y as i32,
                                },
                            );
                            self.redraw = true;
                        }

                        self.mouse_left = *state;
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let pixel_delta = match delta {
                        MouseScrollDelta::LineDelta(_x, y) => y * COMMAND_BUFFER_LINE_HEIGHT,
                        MouseScrollDelta::PixelDelta(PhysicalPosition { x: _, y }) => *y as f32,
                    };

                    if pixel_delta != 0.0 {
                        self.console_buffer.set_scroll(cosmic_text::Scroll {
                            vertical: self.console_buffer.scroll().vertical - pixel_delta,
                            ..self.console_buffer.scroll()
                        });

                        self.redraw = true;
                    }
                }
                _ => {
                    return false;
                }
            }

            if let Some(select) = select {
                let cursor = self.command_editor.cursor();

                let (bound_start, bound_end) = self.command_editor.selection_bounds().unwrap_or_else(|| {
                    let cursor = self.command_editor.cursor();
                    (cursor, cursor)
                });

                match select {
                    /*
                    Select::DeselectNoAffinity => {
                        self.command_editor
                            .set_selection(cosmic_text::Selection::None);
                    }
                     */
                    Select::DeselectPrevAffinity => {
                        if bound_start != bound_end
                            && let Some(cosmic_text::Action::Motion(cosmic_text::Motion::Previous)) = action
                        {
                            action = None;

                            self.command_editor
                                .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::BufferStart));
                            let buffer_start_cursor = self.command_editor.cursor();
                            self.command_editor
                                .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::PreviousWord));
                            let prev_word_cursor = self.command_editor.cursor();

                            self.command_editor.set_cursor(cursor);

                            if bound_start == buffer_start_cursor {
                                self.command_editor.set_cursor(buffer_start_cursor);
                            }

                            if bound_start == prev_word_cursor {
                                self.command_editor.set_cursor(prev_word_cursor);
                            }
                        }

                        self.command_editor.set_selection(cosmic_text::Selection::None);
                    }
                    Select::DeselectNextAffinity => {
                        if bound_start != bound_end
                            && let Some(cosmic_text::Action::Motion(cosmic_text::Motion::Next)) = action
                        {
                            action = None;

                            self.command_editor
                                .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::BufferEnd));
                            let buffer_end_cursor = self.command_editor.cursor();
                            self.command_editor
                                .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::NextWord));
                            let next_word_cursor = self.command_editor.cursor();

                            self.command_editor.set_cursor(cursor);

                            if bound_end == buffer_end_cursor {
                                self.command_editor.set_cursor(buffer_end_cursor);
                            }

                            if bound_end == next_word_cursor {
                                self.command_editor.set_cursor(next_word_cursor);
                            }
                        }

                        self.command_editor.set_selection(cosmic_text::Selection::None);
                    }
                    Select::CurrentCursor => {
                        if bound_start == bound_end {
                            self.command_editor.set_selection(cosmic_text::Selection::Normal(cursor));
                        }
                    }
                }
            }

            if let Some(action) = action {
                self.command_editor.action(&mut self.font_system, action);
                self.refresh_completion();
            }

            if let Some(text) = insert_text {
                for c in text.chars() {
                    self.command_editor.action(&mut self.font_system, cosmic_text::Action::Insert(c));
                }

                let command = self
                    .command_editor
                    .with_buffer_mut(|buffer| cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, ""));
                let mut new_text = command.split_whitespace().collect::<Vec<_>>().join(" ");

                let last_char = command.chars().last().unwrap_or_default();
                if last_char.is_whitespace() {
                    new_text += " "
                }

                self.command_editor.set_cursor(cosmic_text::Cursor {
                    index: self
                        .command_editor
                        .cursor()
                        .index
                        .saturating_add_signed(new_text.len() as isize - command.len() as isize),
                    ..self.command_editor.cursor()
                });

                self.command_editor.with_buffer_mut(|buffer| {
                    cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, &new_text);
                });

                self.refresh_completion();

                self.redraw = true;
            }

            if execute_current_line && self.command_history_pos.is_some() {
                self.command_history.pop_back();
            } else if !self.command_history.is_empty() && self.command_history_pos == Some(self.command_history.len() - 1) {
                self.command_history_pos = None;

                cosmic_text_util::set_command_text(&mut self.command_editor, &mut self.font_system, &self.command_history.pop_back().unwrap());
                self.refresh_completion();
            }

            if execute_current_line {
                let command = cosmic_text_util::set_command_text(&mut self.command_editor, &mut self.font_system, "");
                if !command.is_empty() {
                    self.push_command_history(command.clone());

                    let buffer = cosmic_text_util::set_buffer_text(&mut self.console_buffer, &mut self.font_system, "");

                    match self.command_state.handle_command(game_state, &buffer, &command) {
                        Ok(new_buffer) => {
                            cosmic_text_util::set_buffer_text(&mut self.console_buffer, &mut self.font_system, &new_buffer);
                        }
                        Err(err) => {
                            cosmic_text_util::set_buffer_text(&mut self.console_buffer, &mut self.font_system, &format!("[Error] {err}\n{buffer}"));
                        }
                    }
                }
                self.refresh_completion();
            }
            return true;
        }

        false
    }
}

fn ctrl_key(modifiers: Modifiers) -> bool {
    #[cfg(target_os = "macos")]
    {
        modifiers.state().super_key()
    }
    #[cfg(not(target_os = "macos"))]
    {
        modifiers.state().control_key()
    }
}

mod cosmic_text_util {
    use core::cmp;

    use cosmic_text::Edit;
    use unicode_segmentation::UnicodeSegmentation;

    use crate::debug::GlyphCache;

    pub fn set_buffer_text(buffer: &mut cosmic_text::Buffer, font_system: &mut cosmic_text::FontSystem, new_text: &str) -> String {
        let text = std::mem::take(&mut buffer.lines)
            .into_iter()
            .map(|v| v.into_text())
            .collect::<Vec<_>>()
            .join("\n");

        buffer.set_text(
            font_system,
            new_text,
            &cosmic_text::Attrs::new(),
            cosmic_text::Shaping::Advanced,
            Some(cosmic_text::Align::Left),
        );

        text
    }

    pub fn set_command_text(command_editor: &mut cosmic_text::Editor, font_system: &mut cosmic_text::FontSystem, new_command: &str) -> String {
        command_editor.set_cursor(cosmic_text::Cursor::default());
        command_editor.set_selection(cosmic_text::Selection::None);

        let text = command_editor.with_buffer_mut(|buffer| set_buffer_text(buffer, font_system, new_command));

        command_editor.set_cursor(cosmic_text::Cursor::new(
            0,
            command_editor.with_buffer(|buffer| buffer.lines[0].text().len()),
        ));

        text
    }

    fn cursor_glyph_opt(cursor: &cosmic_text::Cursor, run: &cosmic_text::LayoutRun) -> Option<(usize, f32)> {
        if cursor.line == run.line_i {
            for (glyph_i, glyph) in run.glyphs.iter().enumerate() {
                if cursor.index == glyph.start {
                    return Some((glyph_i, 0.0));
                } else if cursor.index > glyph.start && cursor.index < glyph.end {
                    // Guess x offset based on characters
                    let mut before = 0;
                    let mut total = 0;

                    let cluster = &run.text[glyph.start..glyph.end];
                    for (i, _) in cluster.grapheme_indices(true) {
                        if glyph.start + i < cursor.index {
                            before += 1;
                        }
                        total += 1;
                    }

                    let offset = glyph.w * (before as f32) / (total as f32);
                    return Some((glyph_i, offset));
                }
            }
            match run.glyphs.last() {
                Some(glyph) => {
                    if cursor.index == glyph.end {
                        return Some((run.glyphs.len(), 0.0));
                    }
                }
                None => {
                    return Some((0, 0.0));
                }
            }
        }
        None
    }

    fn cursor_position(cursor: &cosmic_text::Cursor, run: &cosmic_text::LayoutRun) -> Option<(i32, i32)> {
        let (cursor_glyph, cursor_glyph_offset) = cursor_glyph_opt(cursor, run)?;
        let x = run.glyphs.get(cursor_glyph).map_or_else(
            || {
                run.glyphs.last().map_or(0, |glyph| {
                    if glyph.level.is_rtl() {
                        glyph.x as i32
                    } else {
                        (glyph.x + glyph.w) as i32
                    }
                })
            },
            |glyph| {
                if glyph.level.is_rtl() {
                    (glyph.x + glyph.w - cursor_glyph_offset) as i32
                } else {
                    (glyph.x + cursor_glyph_offset) as i32
                }
            },
        );

        Some((x, run.line_top as i32))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        buffer: &cosmic_text::Buffer,
        font_system: &mut cosmic_text::FontSystem,
        cache: &mut cosmic_text::SwashCache,
        text_color: cosmic_text::Color,
        pixmap: &mut tiny_skia::PixmapMut,
        glyph_cache: &mut GlyphCache,
        (offset_x, offset_y): (i32, i32),
    ) {
        let pixmap_paint = tiny_skia::PixmapPaint::default();

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical_glyph = glyph.physical((0., 0.), 1.0);

                let glyph_color = glyph.color_opt.unwrap_or(text_color);

                if !glyph_cache.contains_key(&(glyph_color, physical_glyph.cache_key))
                    && let Some(image) = cache.get_image_uncached(font_system, physical_glyph.cache_key)
                {
                    let x = image.placement.left;
                    let y = -image.placement.top;

                    match image.content {
                        cosmic_text::SwashContent::Mask => {
                            let Some(size) = tiny_skia::IntSize::from_wh(image.placement.width, image.placement.height) else {
                                glyph_cache.insert((glyph_color, physical_glyph.cache_key), (x, y, None));
                                continue;
                            };

                            let glyph = tiny_skia::Pixmap::from_vec(
                                image
                                    .data
                                    .into_iter()
                                    .flat_map(|a| {
                                        bytemuck::cast::<_, [u8; 4]>(
                                            tiny_skia::ColorU8::from_rgba(glyph_color.r(), glyph_color.g(), glyph_color.b(), a).premultiply(),
                                        )
                                    })
                                    .collect(),
                                size,
                            )
                            .unwrap();

                            glyph_cache.insert((glyph_color, physical_glyph.cache_key), (x, y, Some(glyph)));
                        }
                        cosmic_text::SwashContent::Color => {
                            let Some(size) = tiny_skia::IntSize::from_wh(image.placement.width, image.placement.height) else {
                                glyph_cache.insert((glyph_color, physical_glyph.cache_key), (x, y, None));
                                continue;
                            };

                            let glyph = tiny_skia::Pixmap::from_vec(
                                image
                                    .data
                                    .chunks_exact(4)
                                    .flat_map(|v| {
                                        let &[r, g, b, a] = v else { unreachable!() };

                                        bytemuck::cast::<_, [u8; 4]>(tiny_skia::ColorU8::from_rgba(r, g, b, a).premultiply())
                                    })
                                    .collect(),
                                size,
                            )
                            .unwrap();

                            glyph_cache.insert((glyph_color, physical_glyph.cache_key), (x, y, Some(glyph)));
                        }
                        cosmic_text::SwashContent::SubpixelMask => todo!(),
                    }
                }

                if let Some((x, y, Some(glyph))) = glyph_cache.get(&(glyph_color, physical_glyph.cache_key)) {
                    pixmap.draw_pixmap(
                        offset_x + physical_glyph.x + x,
                        offset_y + run.line_y as i32 + physical_glyph.y + y,
                        glyph.as_ref(),
                        &pixmap_paint,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }
        }
    }

    /// Draw the editor
    #[allow(clippy::too_many_arguments)]
    pub fn draw_editor(
        editor: &cosmic_text::Editor,
        cursor_color: cosmic_text::Color,
        selection_color: cosmic_text::Color,
        paint: &mut tiny_skia::Paint,
        pixmap: &mut tiny_skia::PixmapMut,
    ) {
        let selection_bounds = editor.selection_bounds();
        editor.with_buffer(|buffer| {
            for run in buffer.layout_runs() {
                let line_i = run.line_i;
                let line_top = run.line_top;
                let line_height = run.line_height;

                // Highlight selection
                if let Some((start, end)) = selection_bounds
                    && line_i >= start.line
                    && line_i <= end.line
                {
                    let mut range_opt = None;
                    for glyph in run.glyphs {
                        // Guess x offset based on characters
                        let cluster = &run.text[glyph.start..glyph.end];
                        let total = cluster.grapheme_indices(true).count();
                        let mut c_x = glyph.x;
                        let c_w = glyph.w / total as f32;
                        for (i, c) in cluster.grapheme_indices(true) {
                            let c_start = glyph.start + i;
                            let c_end = glyph.start + i + c.len();
                            if (start.line != line_i || c_end > start.index) && (end.line != line_i || c_start < end.index) {
                                range_opt = match range_opt.take() {
                                    Some((min, max)) => Some((cmp::min(min, c_x as i32), cmp::max(max, (c_x + c_w) as i32))),
                                    None => Some((c_x as i32, (c_x + c_w) as i32)),
                                };
                            } else if let Some((min, max)) = range_opt.take() {
                                paint.set_color_rgba8(selection_color.r(), selection_color.g(), selection_color.b(), selection_color.a());
                                pixmap.fill_rect(
                                    tiny_skia::Rect::from_xywh(min as f32, line_top, cmp::max(0, max - min) as f32, line_height).unwrap(),
                                    paint,
                                    tiny_skia::Transform::identity(),
                                    None,
                                );
                            }
                            c_x += c_w;
                        }
                    }

                    if run.glyphs.is_empty() && end.line > line_i {
                        // Highlight all of internal empty lines
                        range_opt = Some((0, buffer.size().0.unwrap_or(0.0) as i32));
                    }

                    if let Some((mut min, mut max)) = range_opt.take() {
                        if end.line > line_i {
                            // Draw to end of line
                            if run.rtl {
                                min = 0;
                            } else {
                                max = buffer.size().0.unwrap_or(0.0) as i32;
                            }
                        }
                        paint.set_color_rgba8(selection_color.r(), selection_color.g(), selection_color.b(), selection_color.a());
                        pixmap.fill_rect(
                            tiny_skia::Rect::from_xywh(min as f32, line_top, cmp::max(0, max - min) as f32, line_height).unwrap(),
                            paint,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                    }
                }

                // Draw cursor
                if let Some((x, y)) = cursor_position(&editor.cursor(), &run) {
                    paint.set_color_rgba8(cursor_color.r(), cursor_color.g(), cursor_color.b(), cursor_color.a());
                    pixmap.fill_rect(
                        tiny_skia::Rect::from_xywh(x as f32, y as f32, 1.0, line_height).unwrap(),
                        paint,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }
        });
    }
}
