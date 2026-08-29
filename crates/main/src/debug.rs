use core::{
    cmp::{max, min},
    str::FromStr,
};
use std::{collections::VecDeque, sync::Arc};

use automancy_data::{
    game::generic::Datum,
    math::{UVec2, Vec2},
};
use automancy_game::{
    persistent::map::GameMapId,
    state::{AutomancyGameState, GameDataStorage},
};
use cosmic_text::Edit;
use fuzzy_matcher::FuzzyMatcher;
use hashbrown::HashMap;
use strum::VariantNames;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, Modifiers, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, NamedKey},
};

const TEXT_SIZE: f32 = 20.0;
const TEXT_HEIGHT: f32 = 24.0;
const COMMAND_HISTORY_LEN: usize = 64;

const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const PIXEL_BYTE_SIZE: u32 = 4;

pub const DEBUG_CONSOLE_KEY: NamedKey = NamedKey::F5;

#[derive(Debug, strum::EnumDiscriminants)]
#[strum_discriminants(name(DebugCommandType))]
#[strum_discriminants(derive(strum::EnumString, strum::VariantNames, strum::IntoStaticStr, strum::Display))]
#[strum_discriminants(strum(serialize_all = "snake_case", ascii_case_insensitive))]
enum DebugCommand {
    Help,
    Clear,
    Beep,
    Boop,
    FontLicense(FontLicenseToShow),
    Set(DebugState),
}

#[derive(Debug, strum::EnumString, strum::VariantNames, strum::IntoStaticStr, strum::Display)]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
enum FontLicenseToShow {
    Symbols,
    Monospace,
}

#[derive(Debug, strum::EnumDiscriminants)]
#[strum_discriminants(name(DebugStateType))]
#[strum_discriminants(derive(strum::EnumString, strum::VariantNames, strum::IntoStaticStr, strum::Display))]
#[strum_discriminants(strum(serialize_all = "snake_case", ascii_case_insensitive))]
enum DebugState {
    LoadDebugMap(bool),
    UnlockEverything(bool),
}

struct CommandHandler {
    debug_map_loaded: bool,
    matcher: fuzzy_matcher::skim::SkimMatcherV2,
}

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

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl CommandHandler {
    fn fmt_arg_list(args_list: &[&str]) -> String {
        args_list.iter().map(|v| format!("<{v}>")).collect::<Vec<_>>().join(" ")
    }

    fn find_cursor_token(command: &str, cursor: usize) -> (&str, usize, usize) {
        let mut word_start = 0;
        let mut word_end = command.len();

        let (cursor_prev, cursor_next) = command.split_at(cursor);
        if let Some(index) = cursor_prev.char_indices().rev().find(|(_, c)| c.is_whitespace()).map(|(i, _)| i) {
            word_start = cursor - ((cursor_prev.len() - 1) - index);
        }

        if let Some(index) = cursor_next.char_indices().find(|(_, c)| c.is_whitespace()).map(|(i, _)| i) {
            word_end = cursor + index;
        }

        (&command[word_start..word_end], word_start, word_end)
    }

    fn parse_tokens(command: &str) -> VecDeque<&str> {
        let mut tokens = command.split_whitespace().filter(|s| !s.is_empty()).collect::<VecDeque<_>>();
        let last_char = command.chars().last().unwrap_or_default();

        if tokens.is_empty() || last_char.is_whitespace() {
            tokens.push_back("");
        }

        tokens
    }

    fn parse_command<'a>(tokens: impl Iterator<Item = &'a str>) -> Result<DebugCommand, (CommandParseError, usize)> {
        let mut tokens = tokens.collect::<VecDeque<_>>();
        if tokens.is_empty() {
            return Err((
                CommandParseError::UnknownCommand {
                    command: String::new(),
                },
                0,
            ));
        }

        let total_tokens = tokens.len();
        let command = tokens.pop_front();
        let err = if let Some(ty) = command.and_then(|s| DebugCommandType::from_str(s).ok()) {
            match ty {
                DebugCommandType::Help => {
                    return Ok(DebugCommand::Help);
                },
                DebugCommandType::Clear => {
                    return Ok(DebugCommand::Clear);
                },
                DebugCommandType::Beep => {
                    return Ok(DebugCommand::Beep);
                },
                DebugCommandType::Boop => {
                    return Ok(DebugCommand::Boop);
                },
                DebugCommandType::FontLicense => {
                    if let Some(v) = tokens.pop_front().and_then(|s| FontLicenseToShow::from_str(s).ok()) {
                        return Ok(DebugCommand::FontLicense(v));
                    }

                    CommandParseError::InvalidArgument {
                        loc: "font_to_show",
                        valid_args: FontLicenseToShow::VARIANTS,
                    }
                },
                DebugCommandType::Set => {
                    if let Some(ty) = tokens.pop_front().and_then(|s| DebugStateType::from_str(s).ok()) {
                        match ty {
                            DebugStateType::LoadDebugMap => {
                                const ARGS_LIST: &[&str] = &["value"];

                                if let Some(v) = tokens.pop_front().and_then(|s| bool::from_str(s).ok()) {
                                    return Ok(DebugCommand::Set(DebugState::LoadDebugMap(v)));
                                }

                                CommandParseError::ExpectingBoolAt {
                                    command: vec![DebugCommandType::Set.into(), DebugStateType::LoadDebugMap.into()],
                                    loc: ARGS_LIST[0],
                                    args_list: ARGS_LIST,
                                }
                            },
                            DebugStateType::UnlockEverything => {
                                const ARGS_LIST: &[&str] = &["value"];

                                if let Some(v) = tokens.pop_front().and_then(|s| bool::from_str(s).ok()) {
                                    return Ok(DebugCommand::Set(DebugState::UnlockEverything(v)));
                                }

                                CommandParseError::ExpectingBoolAt {
                                    command: vec![DebugCommandType::Set.into(), DebugStateType::UnlockEverything.into()],
                                    loc: ARGS_LIST[0],
                                    args_list: ARGS_LIST,
                                }
                            },
                        }
                    } else {
                        CommandParseError::InvalidArgument {
                            loc: "state_name",
                            valid_args: DebugStateType::VARIANTS,
                        }
                    }
                },
            }
        } else {
            CommandParseError::UnknownCommand {
                command: command.unwrap_or_default().to_string(),
            }
        };

        Err((err, total_tokens - tokens.len() - 1))
    }

    pub fn get_completion(&self, command: &str, cursor: usize) -> Vec<String> {
        let tokens = CommandHandler::parse_tokens(command);
        let (cursor_token, ..) = CommandHandler::find_cursor_token(command, cursor);
        let cursor_loc = CommandHandler::parse_tokens(&command[..cursor]).len() - 1;

        match CommandHandler::parse_command(tokens.iter().copied()) {
            Ok(_) => vec![],
            Err((err, err_loc)) => {
                if cursor_loc != err_loc {
                    return vec![];
                }

                let mut args = vec![];

                match err {
                    CommandParseError::UnknownCommand {
                        ..
                    } => {
                        args = DebugCommandType::VARIANTS.to_vec();
                    },
                    CommandParseError::InvalidArgument {
                        valid_args, ..
                    } => {
                        args = valid_args.to_vec();
                    },
                    CommandParseError::ExpectingBoolAt {
                        ..
                    } => {
                        args = vec!["true", "false"];
                    },
                }

                if args.is_empty() {
                    return vec![];
                }

                let mut results = vec![];

                for arg in args {
                    if cursor_token.is_empty() {
                        results.push((0, arg.to_string()));
                        continue;
                    }

                    if !(arg.starts_with(cursor_token)) {
                        continue;
                    }

                    let score = self.matcher.fuzzy_match(arg, cursor_token);
                    let score = max(score.unwrap_or(0), 0) as usize;
                    if score > (arg.len() / 2) {
                        results.push((score, arg.to_string()));
                    }
                }
                results.sort_by(|a, b| a.1.cmp(&b.1));
                results.sort_by_key(|v| v.0);

                results.into_iter().map(|v| v.1).collect()
            },
        }
    }

    pub fn handle_command(
        &mut self,
        game_state: &mut AutomancyGameState,
        game_data: &mut GameDataStorage,
        buffer: &str,
        command: &str,
    ) -> Result<String, String> {
        let response;
        match CommandHandler::parse_command(CommandHandler::parse_tokens(command).iter().copied()) {
            Ok(command) => match command {
                DebugCommand::Help => response = format!("Available commands: [{}]", DebugCommandType::VARIANTS.join(", ")),
                DebugCommand::Clear => return Ok(String::new()),
                DebugCommand::Beep => response = "Boop!".to_string(),
                DebugCommand::Boop => response = "Beep!".to_string(),
                DebugCommand::FontLicense(v) => match v {
                    FontLicenseToShow::Symbols => response = automancy_ui::static_fonts::SYMBOLS_FONT_LICENSE.to_string(),
                    FontLicenseToShow::Monospace => response = automancy_ui::static_fonts::MONOSPACE_FONT_LICENSE.to_string(),
                },
                DebugCommand::Set(DebugState::LoadDebugMap(load_debug_map)) => {
                    #[allow(clippy::collapsible_else_if)]
                    if load_debug_map {
                        if !self.debug_map_loaded {
                            response = "Loading debug map...".to_string();
                            game_state.load_map(game_data, GameMapId::Debug);
                        } else {
                            response = "Debug map already loaded!".to_string();
                        }
                    } else {
                        if self.debug_map_loaded {
                            response = "Loading main menu map...".to_string();
                            game_state.load_map(game_data, GameMapId::MainMenu);
                        } else {
                            response = "Debug map wasn't loaded!".to_string();
                        }
                    }

                    self.debug_map_loaded = load_debug_map;
                },
                DebugCommand::Set(DebugState::UnlockEverything(v)) => {
                    game_data.set_map_datum(game_state.resource_man.registry.data_ids.debug_unlock_everything, Datum::Bool(v));
                    response = format!("Set 'core:debug_unlock_everything' to {v}");
                },
            },
            Err((err, err_loc)) => {
                let err_msg = match err {
                    CommandParseError::InvalidArgument {
                        loc,
                        valid_args,
                    } => {
                        format!("invalid argument at <{loc}>, valid args: [{}]", valid_args.join(", "))
                    },
                    CommandParseError::ExpectingBoolAt {
                        command,
                        loc,
                        args_list,
                    } => {
                        format!(
                            "expecting a 'bool' at <{}>, usage: {} {}",
                            loc,
                            command.join(" "),
                            CommandHandler::fmt_arg_list(args_list)
                        )
                    },
                    CommandParseError::UnknownCommand {
                        command,
                    } => {
                        format!(
                            "unknown command '{}', available commands: [{}]",
                            command,
                            DebugCommandType::VARIANTS.join(", "),
                        )
                    },
                };

                return Err(format!("arg{}: {}", err_loc, err_msg));
            },
        }

        Ok(format!("{response}\n{buffer}"))
    }
}

type GlyphCache = HashMap<(cosmic_text::Color, cosmic_text::CacheKey), (i32, i32, Option<tiny_skia::Pixmap>)>;

/// Handles and renders a debug console, triggered by pressing F5. This should only be available in debugging context.
pub struct DebugConsoleState {
    pub active: bool,

    redraw: bool,
    window_size: UVec2,
    texture: Option<wgpu::Texture>,
    texture_view: Option<wgpu::TextureView>,

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
        {
            let db = font_system.db_mut();

            for file in automancy_ui::static_fonts::MONOSPACE_FONT_DIR.files() {
                let ids = db.load_font_source(cosmic_text::fontdb::Source::Binary(Arc::new(file.contents())));

                for id in ids {
                    if let Some(face) = db.face(id)
                        && let Some((family, _)) = face.families.first().cloned()
                    {
                        db.set_sans_serif_family(family.clone());
                        db.set_serif_family(family.clone());
                        db.set_monospace_family(family.clone());
                        db.set_cursive_family(family.clone());
                        db.set_fantasy_family(family.clone());
                        break;
                    }
                }
            }
        }

        let metrics = cosmic_text::Metrics {
            font_size: TEXT_SIZE,
            line_height: TEXT_HEIGHT,
        };

        let mut command_editor = cosmic_text::Editor::new(cosmic_text::Buffer::new(&mut font_system, metrics));
        command_editor.with_buffer_mut(|buffer| buffer.set_wrap(cosmic_text::Wrap::None));

        let console_buffer = cosmic_text::Buffer::new(&mut font_system, metrics);

        let mut this = Self {
            active: false,

            redraw: true,
            // resize later
            window_size: UVec2::zero(),
            texture: None,
            texture_view: None,

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

#[cfg_attr(feature = "profile", profiling::all_functions)]
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
        self.completions = self.command_state.get_completion(
            &self.command_editor.with_buffer(cosmic_text_util::clone_buffer_text),
            self.command_editor.cursor().index,
        );
    }

    fn commit_completion(&mut self) -> bool {
        if let Some(completion) = self.completion_pos.take().and_then(|i| self.completions.get(i)) {
            let command = self.command_editor.with_buffer(cosmic_text_util::clone_buffer_text);
            let (cursor_token, _cursor_token_start, cursor_token_end) =
                CommandHandler::find_cursor_token(&command, self.command_editor.cursor().index);

            let text_to_insert = completion.strip_prefix(cursor_token).unwrap().to_string();

            let mut cursor = self.command_editor.cursor();
            cursor.index = cursor_token_end;
            self.command_editor.set_cursor(cursor);

            self.command_editor.insert_string(&text_to_insert, None);
            self.refresh_completion();

            return true;
        }

        false
    }

    pub fn resize(&mut self, window_size: UVec2, scale_factor: f32, device: &wgpu::Device) {
        if self.window_size != window_size || self.texture.is_none() {
            self.command_editor.with_buffer_mut(|buffer| {
                buffer.set_size(Some(window_size.x as f32), Some(TEXT_HEIGHT));
                buffer.set_metrics(self.metrics.scale(scale_factor));
                buffer.shape_until_scroll(&mut self.font_system, false);
            });
            self.console_buffer
                .set_size(Some(window_size.x as f32), Some((window_size.y as f32 - TEXT_HEIGHT).max(0.0)));
            self.console_buffer.set_metrics(self.metrics.scale(scale_factor));
            self.console_buffer.shape_until_scroll(&mut self.font_system, false);

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
                format: TEXTURE_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }));
            self.texture_view = Some(self.texture.as_ref().unwrap().create_view(&wgpu::TextureViewDescriptor::default()));
            self.redraw = true;
        }

        self.window_size = window_size;
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
    ) {
        if self.redraw {
            self.redraw = false;

            self.console_buffer.shape_until_scroll(&mut self.font_system, false);

            let cursor = self.command_editor.cursor();
            let scroll = self.command_editor.with_buffer(|buffer| buffer.scroll());
            let scroll_offset = -Vec2::new(scroll.horizontal, scroll.vertical);
            self.command_editor.shape_as_needed(&mut self.font_system, false);

            let surface_size = self.window_size;
            let mut pixel_data: Vec<u8> = std::iter::repeat_n([0, 0, 0, 208], surface_size.x as usize * surface_size.y as usize)
                .flatten()
                .collect::<Vec<_>>();

            let mut pixmap = tiny_skia::PixmapMut::from_bytes(&mut pixel_data, surface_size.x, surface_size.y).unwrap();
            let mut paint = tiny_skia::Paint {
                anti_alias: false,
                ..Default::default()
            };

            // draw console text
            cosmic_text_util::draw_text(
                &self.console_buffer,
                &mut self.font_system,
                &mut self.swash_cache,
                cosmic_text::Color::rgb(192, 192, 192),
                &mut pixmap,
                &mut self.glyph_cache,
                Vec2::new(0.0, TEXT_HEIGHT),
            );

            // draw command background
            {
                paint.blend_mode = tiny_skia::BlendMode::Source;
                paint.set_color_rgba8(0, 0, 0, 224);
                pixmap.fill_rect(
                    tiny_skia::Rect::from_xywh(0.0, 0.0, surface_size.x as f32, TEXT_HEIGHT).unwrap(),
                    &paint,
                    tiny_skia::Transform::identity(),
                    None,
                );
                paint.blend_mode = tiny_skia::BlendMode::default();
            }

            let mut command = self.command_editor.with_buffer(cosmic_text_util::clone_buffer_text);
            let mut command_cursor_index = cursor.index;

            // draw completion
            if !self.completions.is_empty()
                && let Some(completion_pos) = self.completion_pos
            {
                self.command_editor.with_buffer(|buffer| {
                    let mut buffer = buffer.clone();

                    let (_cursor_token, cursor_token_start, cursor_token_end) = CommandHandler::find_cursor_token(&command, cursor.index);

                    let current_completion = &self.completions[completion_pos];
                    let other_completions = [
                        self.completions[min(completion_pos + 1, self.completions.len())..].iter(),
                        self.completions[..completion_pos].iter(),
                    ]
                    .into_iter()
                    .flatten();

                    // measure widths
                    let current_completion_width = {
                        cosmic_text_util::set_buffer_text(&mut buffer, &mut self.font_system, current_completion);

                        buffer
                            .line_layout(&mut self.font_system, cursor.line)
                            .and_then(|layouts| layouts.iter().map(|line| line.w).max_by(|a, b| a.partial_cmp(b).unwrap()))
                            .unwrap_or_default()
                    };
                    let max_completion_width = {
                        let mut max_width: f32 = 0.0;

                        for completion_text in &self.completions {
                            cosmic_text_util::set_buffer_text(&mut buffer, &mut self.font_system, completion_text);

                            max_width = max_width.max(
                                buffer
                                    .line_layout(&mut self.font_system, cursor.line)
                                    .and_then(|layouts| layouts.iter().map(|line| line.w).max_by(|a, b| a.partial_cmp(b).unwrap()))
                                    .unwrap_or_default(),
                            );
                        }

                        max_width
                    };
                    let width_before_current_token = {
                        cosmic_text_util::set_buffer_text(&mut buffer, &mut self.font_system, &command[..cursor_token_start]);

                        buffer
                            .line_layout(&mut self.font_system, cursor.line)
                            .and_then(|layouts| layouts.iter().map(|line| line.w).max_by(|a, b| a.partial_cmp(b).unwrap()))
                            .unwrap_or_default()
                    };

                    {
                        // draw current completion background
                        paint.blend_mode = tiny_skia::BlendMode::Source;
                        paint.set_color_rgba8(0, 0, 0, 255);
                        pixmap.fill_rect(
                            tiny_skia::Rect::from_xywh(
                                scroll_offset.x + width_before_current_token,
                                scroll_offset.y,
                                current_completion_width,
                                TEXT_HEIGHT,
                            )
                            .unwrap(),
                            &paint,
                            tiny_skia::Transform::identity(),
                            None,
                        );

                        // draw other completions background
                        paint.set_color_rgba8(0, 0, 0, 240);
                        pixmap.fill_rect(
                            tiny_skia::Rect::from_xywh(
                                scroll_offset.x + width_before_current_token,
                                scroll_offset.y + TEXT_HEIGHT,
                                max_completion_width,
                                TEXT_HEIGHT * (self.completions.len() - 1) as f32,
                            )
                            .unwrap(),
                            &paint,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                        paint.blend_mode = tiny_skia::BlendMode::default();
                    }

                    // draw other completions
                    for (index, completion_text) in other_completions.enumerate() {
                        cosmic_text_util::set_buffer_text(&mut buffer, &mut self.font_system, completion_text);
                        cosmic_text_util::draw_text(
                            &buffer,
                            &mut self.font_system,
                            &mut self.swash_cache,
                            cosmic_text::Color::rgb(160, 160, 160),
                            &mut pixmap,
                            &mut self.glyph_cache,
                            scroll_offset + Vec2::new(width_before_current_token, (index + 1) as f32 * TEXT_HEIGHT),
                        );
                    }

                    command = format!(
                        "{}{}{}",
                        &command[..cursor_token_start],
                        current_completion,
                        &command[cursor_token_end..]
                    );
                    command_cursor_index = cursor_token_start + self.completions.iter().map(|s| s.len()).max().unwrap_or_default();
                });
            }

            // draw command text

            {
                let curr_command = self.command_editor.with_buffer(cosmic_text_util::clone_buffer_text);
                let mut scroll = scroll;

                if curr_command != command {
                    let old_scroll = self.command_editor.with_buffer(|buffer| buffer.scroll());
                    self.command_editor
                        .with_buffer_mut(|buffer| cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, &command));

                    let mut cursor = cursor;
                    cursor.index = command_cursor_index.min(command.len());
                    self.command_editor.set_cursor(cursor);
                    self.command_editor.shape_as_needed(&mut self.font_system, false);
                    scroll = self.command_editor.with_buffer(|buffer| buffer.scroll());

                    if old_scroll != scroll {
                        self.redraw = true;
                    }
                }

                cosmic_text_util::draw_editor(
                    &self.command_editor,
                    &mut self.font_system,
                    &mut self.swash_cache,
                    cosmic_text::Color::rgb(255, 255, 255),
                    cosmic_text::Color::rgb(255, 36, 36),
                    cosmic_text::Color::rgba(181, 210, 255, 127),
                    &mut pixmap,
                    &mut self.glyph_cache,
                    &mut paint,
                );

                if curr_command != command {
                    self.command_editor.with_buffer_mut(|buffer| {
                        cosmic_text_util::set_buffer_text(buffer, &mut self.font_system, &curr_command);
                        buffer.set_scroll(scroll);
                    });
                    self.command_editor.set_cursor(cursor);
                }
            }

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
                    bytes_per_row: Some(surface_size.width * PIXEL_BYTE_SIZE),
                    rows_per_image: Some(surface_size.height),
                },
                surface_size,
            );
        }

        // recursively redraw if drawing gets outdated mid-drawing
        if self.redraw {
            return self.draw(device, queue, encoder, surface_view);
        }

        if let Some(texture_view) = &self.texture_view {
            wgpu::util::TextureBlitterBuilder::new(device, surface_view.texture().format())
                .blend_state(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING)
                .build()
                .copy(device, encoder, texture_view, surface_view);
        }
    }

    pub fn handle_event(
        &mut self,
        game_state: &mut AutomancyGameState,
        game_data: &mut GameDataStorage,
        event: &WindowEvent,
        clipboard: &mut arboard::Clipboard,
    ) -> bool {
        enum Select {
            DeselectPrevAffinity,
            DeselectNextAffinity,
            CurrentCursor,
        }

        let mut execute_current_line = false;
        let mut insert_text = None;

        let mut action = None;
        let mut select = None;
        let scroll_offset = self
            .command_editor
            .with_buffer(|buffer| Vec2::new(buffer.scroll().horizontal, buffer.scroll().vertical));

        match event {
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = *modifiers,
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    logical_key,
                    state,
                    ..
                },
                ..
            } if state.is_pressed() => {
                #[allow(clippy::collapsible_match)]
                match logical_key {
                    Key::Named(NamedKey::Escape) => {
                        if self.completion_pos.is_some() {
                            self.completion_pos = None;
                        } else {
                            self.active = false
                        }
                    },
                    Key::Named(NamedKey::Enter) => {
                        if !self.commit_completion() {
                            execute_current_line = true;
                        }
                    },
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
                    },
                    Key::Named(NamedKey::Space) if self.completion_pos.is_some() => {
                        self.commit_completion();
                    },
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
                    },
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
                    },
                    Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::PageUp) => {
                        if let Some(pos) = self.completion_pos {
                            self.completion_pos = Some((pos as isize - 1).rem_euclid(self.completions.len() as isize) as usize)
                        } else if self.modifiers.state().shift_key() || ctrl_key(self.modifiers) {
                            self.console_buffer.set_scroll(cosmic_text::Scroll {
                                vertical: self.console_buffer.scroll().vertical - TEXT_HEIGHT,
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

                                cosmic_text_util::set_command_text(
                                    &mut self.command_editor,
                                    &mut self.font_system,
                                    &self.command_history[*pos],
                                );
                                self.refresh_completion();
                            }
                        }
                    },
                    Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::PageDown) => {
                        if let Some(pos) = self.completion_pos {
                            self.completion_pos = Some((pos + 1) % self.completions.len())
                        } else if self.modifiers.state().shift_key() || ctrl_key(self.modifiers) {
                            self.console_buffer.set_scroll(cosmic_text::Scroll {
                                vertical: self.console_buffer.scroll().vertical + TEXT_HEIGHT,
                                ..self.console_buffer.scroll()
                            });
                        } else if let Some(pos) = &mut self.command_history_pos {
                            *pos = min(*pos + 1, self.command_history.len() - 1);

                            cosmic_text_util::set_command_text(
                                &mut self.command_editor,
                                &mut self.font_system,
                                &self.command_history[*pos],
                            );
                            self.refresh_completion();
                        }
                    },
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
                    },
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
                    },
                    Key::Named(NamedKey::Backspace) => {
                        if self.completion_pos.is_some() {
                            self.completion_pos = None;
                        } else if self.command_editor.selection_bounds().is_none() && ctrl_key(self.modifiers) {
                            self.command_editor
                                .set_selection(cosmic_text::Selection::Normal(self.command_editor.cursor()));
                            self.command_editor.action(
                                &mut self.font_system,
                                cosmic_text::Action::Motion(cosmic_text::Motion::PreviousWord),
                            );
                            self.command_editor.delete_selection();
                            self.refresh_completion();
                        } else {
                            action = Some(cosmic_text::Action::Backspace);
                        }
                    },
                    Key::Named(NamedKey::Delete) => {
                        action = Some(cosmic_text::Action::Delete);
                    },

                    Key::Character(key) if key.to_lowercase() == "a" && ctrl_key(self.modifiers) => {
                        self.command_editor
                            .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::BufferStart));
                        self.command_editor
                            .set_selection(cosmic_text::Selection::Normal(self.command_editor.cursor()));
                        self.command_editor
                            .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::BufferEnd));
                    },
                    Key::Character(key) if key.to_lowercase() == "x" && ctrl_key(self.modifiers) => {
                        if let Some(text) = self.command_editor.copy_selection() {
                            clipboard.set_text(&text).unwrap();
                        }
                        self.command_editor.delete_selection();
                        self.refresh_completion();
                    },
                    Key::Character(key) if key.to_lowercase() == "c" && ctrl_key(self.modifiers) => {
                        if let Some(text) = self.command_editor.copy_selection() {
                            clipboard.set_text(&text).unwrap();
                        }
                    },
                    Key::Character(key) if key.to_lowercase() == "v" && ctrl_key(self.modifiers) => {
                        if let Ok(text) = clipboard.get_text() {
                            insert_text = Some(text);
                        }
                    },

                    Key::Named(key) if !ctrl_key(self.modifiers) => {
                        insert_text = key.to_text().map(str::to_string);
                    },
                    Key::Character(text) if !ctrl_key(self.modifiers) => {
                        insert_text = Some(text.as_str().to_string());
                    },

                    _ => {},
                }

                self.redraw = true;
            },

            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                insert_text = Some(text.clone());

                self.redraw = true;
            },
            WindowEvent::CursorMoved {
                position, ..
            } => {
                self.mouse_x = position.x as f32;
                self.mouse_y = position.y as f32;

                if self.mouse_left.is_pressed() && !self.modifiers.state().shift_key() {
                    self.command_editor.action(
                        &mut self.font_system,
                        cosmic_text::Action::Drag {
                            x: (scroll_offset.x + self.mouse_x) as i32,
                            y: (scroll_offset.y + self.mouse_y) as i32,
                        },
                    );
                    self.refresh_completion();
                    self.redraw = true;
                }
            },
            WindowEvent::MouseInput {
                state,
                button,
                ..
            } => {
                if *button == MouseButton::Left {
                    if *state == ElementState::Pressed && self.mouse_left == ElementState::Released {
                        if self.modifiers.state().shift_key() {
                            let cursor = self.command_editor.cursor();
                            self.command_editor.action(
                                &mut self.font_system,
                                cosmic_text::Action::Click {
                                    x: (scroll_offset.x + self.mouse_x) as i32,
                                    y: (scroll_offset.y + self.mouse_y) as i32,
                                },
                            );
                            self.command_editor
                                .set_selection(cosmic_text::Selection::Normal(self.command_editor.cursor()));
                            self.command_editor.set_cursor(cursor);
                        } else {
                            self.command_editor.set_selection(cosmic_text::Selection::None);
                            self.command_editor.action(
                                &mut self.font_system,
                                cosmic_text::Action::Click {
                                    x: (scroll_offset.x + self.mouse_x) as i32,
                                    y: (scroll_offset.y + self.mouse_y) as i32,
                                },
                            );
                        }
                        self.refresh_completion();
                        self.redraw = true;
                    }

                    self.mouse_left = *state;
                }
            },
            WindowEvent::MouseWheel {
                delta, ..
            } => {
                let pixel_delta = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y * TEXT_HEIGHT,
                    MouseScrollDelta::PixelDelta(PhysicalPosition {
                        x: _,
                        y,
                    }) => *y as f32,
                };

                if pixel_delta != 0.0 {
                    self.console_buffer.set_scroll(cosmic_text::Scroll {
                        vertical: self.console_buffer.scroll().vertical - pixel_delta,
                        ..self.console_buffer.scroll()
                    });
                    self.redraw = true;
                }
            },
            _ => {
                return false;
            },
        }

        if let Some(select) = select {
            let cursor = self.command_editor.cursor();

            let (bound_start, bound_end) = self.command_editor.selection_bounds().unwrap_or_else(|| {
                let cursor = self.command_editor.cursor();
                (cursor, cursor)
            });

            match select {
                Select::DeselectPrevAffinity => {
                    if bound_start != bound_end && action == Some(cosmic_text::Action::Motion(cosmic_text::Motion::Previous)) {
                        action = None;

                        self.command_editor
                            .action(&mut self.font_system, cosmic_text::Action::Motion(cosmic_text::Motion::BufferStart));
                        let buffer_start_cursor = self.command_editor.cursor();
                        self.command_editor.action(
                            &mut self.font_system,
                            cosmic_text::Action::Motion(cosmic_text::Motion::PreviousWord),
                        );
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
                },
                Select::DeselectNextAffinity => {
                    if bound_start != bound_end && action == Some(cosmic_text::Action::Motion(cosmic_text::Motion::Next)) {
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
                },
                Select::CurrentCursor => {
                    if bound_start == bound_end {
                        self.command_editor.set_selection(cosmic_text::Selection::Normal(cursor));
                    }
                },
            }
        }

        if let Some(action) = action {
            self.command_editor.action(&mut self.font_system, action);
            self.refresh_completion();
        }

        if let Some(text) = insert_text {
            for c in text.chars() {
                let cursor = self.command_editor.cursor();
                let should_skip = c.is_whitespace()
                    && self.command_editor.with_buffer(|buffer| {
                        let mut line = buffer.lines[cursor.line].clone();
                        let after = line.split_off(cursor.index);

                        if let Some(char) = line.text().chars().last()
                            && char.is_whitespace()
                        {
                            return true;
                        } else if let Some(char) = after.text().chars().next()
                            && char.is_whitespace()
                        {
                            return true;
                        }

                        false
                    });

                if !should_skip {
                    self.command_editor.action(&mut self.font_system, cosmic_text::Action::Insert(c));
                }
            }

            self.refresh_completion();

            self.redraw = true;
        }

        if execute_current_line && self.command_history_pos.is_some() {
            self.command_history.pop_back();
        } else if !self.command_history.is_empty() && self.command_history_pos == Some(self.command_history.len() - 1) {
            self.command_history_pos = None;

            cosmic_text_util::set_command_text(
                &mut self.command_editor,
                &mut self.font_system,
                &self.command_history.pop_back().unwrap(),
            );
            self.refresh_completion();
        }

        if execute_current_line {
            let command = cosmic_text_util::set_command_text(&mut self.command_editor, &mut self.font_system, "");
            if !command.is_empty() {
                self.push_command_history(command.clone());

                let buffer = cosmic_text_util::clone_buffer_text(&self.console_buffer);
                match self.command_state.handle_command(game_state, game_data, &buffer, &command) {
                    Ok(new_buffer) => {
                        cosmic_text_util::set_buffer_text(&mut self.console_buffer, &mut self.font_system, &new_buffer);
                    },
                    Err(err) => {
                        cosmic_text_util::set_buffer_text(
                            &mut self.console_buffer,
                            &mut self.font_system,
                            &format!("[Error] {err}\n{buffer}"),
                        );
                    },
                }
            }
            self.refresh_completion();
        }

        true
    }
}

#[inline]
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
    use automancy_data::math::Vec2;
    use cosmic_text::Edit;

    use crate::debug::GlyphCache;

    #[inline]
    pub fn clone_buffer_text(buffer: &cosmic_text::Buffer) -> String {
        buffer.lines.iter().map(|v| v.text()).collect::<Vec<_>>().join("\n")
    }

    #[inline]
    pub fn set_buffer_text(buffer: &mut cosmic_text::Buffer, font_system: &mut cosmic_text::FontSystem, new_text: &str) -> String {
        let text = std::mem::take(&mut buffer.lines)
            .into_iter()
            .map(|v| v.into_text())
            .collect::<Vec<_>>()
            .join("\n");

        buffer.set_text(
            new_text,
            &cosmic_text::Attrs::new(),
            cosmic_text::Shaping::Advanced,
            Some(cosmic_text::Align::Left),
        );
        buffer.shape_until_scroll(font_system, false);

        text
    }

    #[inline]
    pub fn set_command_text(
        command_editor: &mut cosmic_text::Editor,
        font_system: &mut cosmic_text::FontSystem,
        new_command: &str,
    ) -> String {
        command_editor.set_cursor(cosmic_text::Cursor::default());
        command_editor.set_selection(cosmic_text::Selection::None);

        let text = command_editor.with_buffer_mut(|buffer| set_buffer_text(buffer, font_system, new_command));

        command_editor.set_cursor(cosmic_text::Cursor::new(
            0,
            command_editor.with_buffer(|buffer| buffer.lines[0].text().len()),
        ));

        text
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn draw_text(
        buffer: &cosmic_text::Buffer,
        font_system: &mut cosmic_text::FontSystem,
        cache: &mut cosmic_text::SwashCache,
        text_color: cosmic_text::Color,
        pixmap: &mut tiny_skia::PixmapMut,
        glyph_cache: &mut GlyphCache,
        offset: Vec2,
    ) {
        let pixmap_paint = tiny_skia::PixmapPaint::default();

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical_glyph = glyph.physical((offset.x, offset.y + run.line_y), 1.0);

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
                                            tiny_skia::ColorU8::from_rgba(glyph_color.r(), glyph_color.g(), glyph_color.b(), a)
                                                .premultiply(),
                                        )
                                    })
                                    .collect(),
                                size,
                            )
                            .unwrap();

                            glyph_cache.insert((glyph_color, physical_glyph.cache_key), (x, y, Some(glyph)));
                        },
                        cosmic_text::SwashContent::Color => {
                            let Some(size) = tiny_skia::IntSize::from_wh(image.placement.width, image.placement.height) else {
                                glyph_cache.insert((glyph_color, physical_glyph.cache_key), (x, y, None));
                                continue;
                            };

                            let glyph = tiny_skia::Pixmap::from_vec(
                                image
                                    .data
                                    .as_chunks::<4>()
                                    .0
                                    .iter()
                                    .flat_map(|&[r, g, b, a]| {
                                        bytemuck::cast::<_, [u8; 4]>(tiny_skia::ColorU8::from_rgba(r, g, b, a).premultiply())
                                    })
                                    .collect(),
                                size,
                            )
                            .unwrap();

                            glyph_cache.insert((glyph_color, physical_glyph.cache_key), (x, y, Some(glyph)));
                        },
                        cosmic_text::SwashContent::SubpixelMask => todo!(),
                    }
                }

                if let Some((x, y, Some(glyph))) = glyph_cache.get(&(glyph_color, physical_glyph.cache_key)) {
                    pixmap.draw_pixmap(
                        physical_glyph.x + x,
                        physical_glyph.y + y,
                        glyph.as_ref(),
                        &pixmap_paint,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }
        }
    }

    #[cfg_attr(feature = "profile", profiling::function)]
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn draw_editor(
        editor: &cosmic_text::Editor,
        font_system: &mut cosmic_text::FontSystem,
        cache: &mut cosmic_text::SwashCache,
        text_color: cosmic_text::Color,
        cursor_color: cosmic_text::Color,
        selection_color: cosmic_text::Color,
        pixmap: &mut tiny_skia::PixmapMut,
        glyph_cache: &mut GlyphCache,
        paint: &mut tiny_skia::Paint,
    ) {
        let selection_bounds = editor.selection_bounds();
        editor.with_buffer(|buffer| {
            let scroll_offset = -Vec2::new(buffer.scroll().horizontal, buffer.scroll().vertical);

            for run in buffer.layout_runs() {
                let line_i = run.line_i;
                let line_top = run.line_top;
                let line_height = run.line_height;

                // Highlight selection
                if let Some((start, end)) = selection_bounds
                    && line_i >= start.line
                    && line_i <= end.line
                {
                    for (x, w) in run.highlight(start, end) {
                        paint.set_color_rgba8(selection_color.r(), selection_color.g(), selection_color.b(), selection_color.a());
                        pixmap.fill_rect(
                            tiny_skia::Rect::from_xywh(scroll_offset.x + x, scroll_offset.y + line_top, w, line_height).unwrap(),
                            paint,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                    }
                }

                // Draw cursor
                if let Some((x, y)) = editor.cursor_position() {
                    paint.set_color_rgba8(cursor_color.r(), cursor_color.g(), cursor_color.b(), cursor_color.a());
                    pixmap.fill_rect(
                        tiny_skia::Rect::from_xywh(scroll_offset.x + x as f32 - 1.0, scroll_offset.y + y as f32, 1.0, line_height).unwrap(),
                        paint,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }

            draw_text(buffer, font_system, cache, text_color, pixmap, glyph_cache, scroll_offset);
        });
    }
}
