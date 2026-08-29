#![feature(string_into_chars)]
#![feature(const_trait_impl)]

#[allow(unused)]
mod components;

mod gui;
mod ignore_debug;
mod script;
mod styling;
mod traits;
mod util;

pub(crate) use yakui_shorthands::*;
mod yakui_shorthands {
    #[allow(unused)]
    pub use yakui::{
        align, colored_box, colored_box_container, colored_circle, constrained, divider, draggable, offset, reflow, spacer, stack,
        widgets::{
            Button, ButtonResponse, Circle, ColoredBox, ConstrainedBox, CountGrid, DynamicButtonStyle, Flexible, Image, ImageFit, Layer,
            List, Opaque, Pad, Text, TextBox, TextBoxResponse, TextResponse, measure_text_width,
        },
    };
}

pub(crate) use prelude::*;
mod prelude {
    #[allow(unused)]
    pub use core::{
        cell::{Cell, Ref, RefCell, RefMut, UnsafeCell},
        fmt::Debug,
        hash::Hash,
        marker::PhantomData,
        ops::{Deref, DerefMut, RangeInclusive},
        str::FromStr,
        time::Duration,
    };
    pub use std::{
        borrow::Cow,
        collections::{BTreeMap, BTreeSet},
        fs,
        rc::Rc,
        sync::Arc,
        time::Instant,
    };

    pub use automancy_data::{
        game::{
            coord::{TileCoord, layout::TileLayout},
            generic::{DataMap, Datum},
            inventory::{Inventory, ItemStack},
        },
        id::*,
        math,
        math::RectExt,
        rendering::{
            self, GenericModel, colors,
            colors::{ColorExt, PackedRgba},
            draw::GameDrawInstance,
        },
    };
    #[allow(unused)]
    pub use automancy_game::{
        actor::{FlatTile, FlatTiles, TileEntry, TileMap, message::*, tile_entity},
        input::*,
        persistent::{
            self,
            map::{GameMap, GameMapId, SaveFileName},
            options,
        },
        pkg,
        resources::ResourceManager,
        state::{
            AutomancyGameLoadResult, AutomancyGameState, GameDataStorage,
            error::{AutomancyError, ErrorManager},
        },
    };
    pub use automancy_rendering::{AnimationChannel, AnimationFrame, Interpolation, gpu, renderer};
    pub use chrono::{DateTime, Local};
    pub use easing_function::{Easing, EasingFunction, easings};
    pub use fuzzy_matcher::FuzzyMatcher;
    pub use glam::{FloatExt, Vec2Swizzles};
    pub use hashbrown::HashMap;
    pub use interpolator::Formattable;
    pub use ractor::ActorRef;
    pub use vek::Clamp;
    pub use winit::{
        keyboard::{Key, NamedKey},
        window::Window as WinitWindow,
    };
    #[allow(unused)]
    pub use yakui::{
        Alignment, Border, BorderRadius, Color, Constraints, CrossAxisAlignment, Dim, Dim2, Direction, FlexFit, Flow, MainAxisAlignItems,
        MainAxisAlignment, MainAxisSize, Pivot, Rect, Response, URect, UVec2, Vec2, Vec4, WidgetId, Yakui, context, cosmic_text,
        event::*,
        font::*,
        input::*,
        layout::*,
        paint::{PaintCall, UserPaintCallId},
        style::*,
        util::{widget, *},
        widget::{Widget, *},
    };
    pub use yakui_wgpu::{Buffers as YakuiBuffers, YakuiWgpu};
    pub use yakui_winit::YakuiWinit;

    pub use crate::{AutomancyUiContext as UiContext, components::*, ignore_debug::*, script::*, state::*, styling::*, traits::*, util::*};
}

pub(crate) mod custom;

#[allow(unused)]
pub(crate) mod shapes;

#[allow(unused)]
pub(crate) mod sizing;

pub mod state;
pub mod static_fonts;

#[macro_export]
macro_rules! gui_str {
    ($game_state:expr, $id:ident) => {
        $game_state
            .resource_man
            .gui_str($game_state.resource_man.registry.gui_ids.$id)
            .clone()
    };

    ($game_state:expr, $id:ident, $fmt:expr) => {
        $game_state
            .resource_man
            .gui_fmt($game_state.resource_man.registry.gui_ids.$id, $fmt)
            .clone()
    };
}

pub struct AutomancyUiContext<'a> {
    pub gui: &'a mut AutomancyGui,

    pub render: &'a mut renderer::AutomancyRendering,
    pub render_state: &'a mut renderer::AutomancyRenderState,

    pub game_state: &'a mut AutomancyGameState,
    pub game_data: &'a mut GameDataStorage,

    pub winit_window: &'a WinitWindow,
    pub closing: &'a mut bool,
}

pub struct AutomancyGui {
    pub state: UiState,
    pub prev_state: UiState,
    pub font_families: StaticList<Cow<'static, str>>,
    using_system_font: Option<bool>,
    system_sans_serif_font: Option<String>,
    pub logo: yakui::TextureId,

    pub puzzle_state: Option<(DataMap, bool)>,

    pub yak: Yakui,
    pub yakui_buffers: YakuiBuffers,
    pub yakui_wgpu: YakuiWgpu,
    pub yakui_winit: YakuiWinit,

    pub focus_next_frame: FocusState,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl AutomancyGui {
    fn cleanup_fonts(&mut self) {
        self.font_families = Default::default();
        self.using_system_font = None;
        self.system_sans_serif_font = None;

        let text_state = self.yak.dom().get_global_or_init(yakui::text_renderer::TextGlobalState::new);
        text_state.fonts_changed();

        let fonts = self.yak.dom().get_global_or_init(Fonts::default);
        fonts.with_inner(|fonts| fonts.clear_fonts());
        fonts.set_serif_family("");
        fonts.set_sans_serif_family("");
        fonts.set_cursive_family("");
        fonts.set_fantasy_family("");
        fonts.set_monospace_family("");
    }

    pub fn use_system_fonts(&mut self) {
        if self.using_system_font == Some(true) {
            return;
        }
        self.cleanup_fonts();

        log::info!("Loading system fonts...");

        let fonts = self.yak.dom().get_global_or_init(Fonts::default);
        static_fonts::load_static_fonts(&fonts);

        fonts.with_inner(|fonts| fonts.load_system_fonts());
        self.system_sans_serif_font = Some(fonts.with_inner(|fonts| fonts.font_selection.sans_serif_family.to_string()));

        let font_families = fonts.with_inner(|fonts| {
            fonts
                .font_selection
                .font_families()
                .map(|s| Cow::Borrowed(s.to_string().leak()))
                .collect::<Vec<_>>()
        });
        self.font_families = StaticList::rc(font_families);

        log::info!("Loaded system fonts.");
        self.using_system_font = Some(true);
    }

    pub fn use_built_in_fonts(&mut self, resource_man: &ResourceManager) {
        if self.using_system_font == Some(false) {
            return;
        }
        self.cleanup_fonts();

        log::info!("Loading built-in fonts...");

        let fonts = self.yak.dom().get_global_or_init(Fonts::default);
        static_fonts::load_static_fonts(&fonts);

        let mut font_families = vec![];
        for (font_family, font_data_list) in &resource_man.fonts {
            for font_data in font_data_list {
                fonts.load_font_source(cosmic_text::fontdb::Source::Binary(font_data.bytes.clone()));
            }
            // TODO leak at loading instead
            font_families.push(Cow::Borrowed(font_family.clone().leak()));
        }
        self.font_families = StaticList::rc(font_families);

        log::info!("Loaded built-in fonts.");
        self.using_system_font = Some(false);
    }

    pub fn set_font_family<S: Into<Cow<'static, str>>>(&mut self, font_family: S) {
        let font_family = font_family.into();
        log::info!("Setting font to {font_family}...");

        let fonts = self.yak.dom().get_global_or_init(Fonts::default);
        fonts.set_sans_serif_family(font_family.clone());

        log::info!("Set font to {font_family}.");
    }

    pub fn set_font_family_system(&mut self) {
        log::info!("Setting fonts to system preferences...");

        self.set_font_family(self.system_sans_serif_font.clone().unwrap());
    }

    pub fn viewport(&self) -> Vec2 {
        self.yak.layout_dom().viewport().size()
    }

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, window: &WinitWindow, logo: yakui::paint::Texture) -> Self {
        let mut yak = Yakui::new();

        let yakui_wgpu = yakui_wgpu::YakuiWgpu::new(device.clone(), queue.clone());
        let mut yakui_winit = yakui_winit::YakuiWinit::new(window);

        yakui_winit.set_automatic_scale_factor(false);

        Self {
            state: UiState::default(),
            prev_state: UiState::default(),
            font_families: Default::default(),
            using_system_font: None,
            system_sans_serif_font: None,
            logo: yakui::TextureId::Managed(yak.add_texture(logo)),

            puzzle_state: None,

            yak,
            yakui_buffers: yakui_wgpu.buffers(),
            yakui_wgpu,
            yakui_winit,

            focus_next_frame: FocusState::None,
        }
    }
}

#[inline]
pub fn layout(ctx: &mut UiContext) {
    ctx.gui.yak.start();
    crate::gui::layout_ui(ctx);
    ctx.gui.yak.finish();

    match std::mem::take(&mut ctx.gui.focus_next_frame) {
        FocusState::None => {},
        FocusState::Clear => {
            ctx.gui.yak.request_focus(None);
        },
        FocusState::Set(id) => {
            ctx.gui.yak.request_focus(Some(id));
        },
    }

    ctx.gui.prev_state = ctx.gui.state.clone();
}

#[inline]
pub fn render(
    gui: &mut AutomancyGui,
    render: &mut renderer::AutomancyRendering,
    render_state: &mut renderer::AutomancyRenderState,
    resource_man: &ResourceManager,
    encoder: &mut wgpu::CommandEncoder,
) {
    use yakui_wgpu::DrawCall;

    // --- yakui ---
    gui.yakui_wgpu.set_paint_limits(&mut gui.yak);
    let paint = gui.yak.paint();

    gui.yakui_wgpu.update_textures(paint);

    // If there's nothing to paint, well... don't paint!
    let layers = &paint.layers;
    if layers.iter().all(|layer| layer.calls.is_empty()) {
        return;
    }

    // If the surface has a size of zero, well... don't paint either!
    if paint.surface_size().x == 0.0 || paint.surface_size().y == 0.0 {
        return;
    }

    gui.yakui_buffers.vertices.clear();
    gui.yakui_buffers.indices.clear();
    gui.yakui_wgpu.texture_bindgroup_cache.clear();

    let mut draw_calls = Vec::with_capacity(layers.len());
    // --- yakui ---

    {
        let custom = paint.globals.get_mut().get_mut(custom::CustomRenderer::default);
        custom.atlas_man.update_textures(&mut gui.yakui_wgpu, &mut render.res);
    }

    for (clip, call) in layers.iter().flat_map(|layer| &layer.calls) {
        match call {
            PaintCall::Internal(call) => {
                draw_calls.push(gui.yakui_wgpu.build_draw_call(&mut gui.yakui_buffers, *clip, call));
            },
            PaintCall::User(id) => {
                draw_calls.push((*clip, DrawCall::User(*id)));
            },
        }
    }

    let surface = yakui_wgpu::SurfaceInfo {
        format: render.res.gui_res.gui_texture.format(),
        sample_count: render.res.gui_res.gui_texture_mxaa.sample_count(),
        color_attachment: wgpu::RenderPassColorAttachment {
            view: &render.res.gui_res.gui_texture_mxaa_view,
            depth_slice: None,
            resolve_target: Some(&render.res.gui_res.gui_texture_view),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        },
    };

    {
        let custom = paint.globals.get_mut().get_mut(custom::CustomRenderer::default);
        custom
            .game_model_renderer
            .update_pipelines(&render.res, &custom.atlas_man, &surface);
        for atlas in custom.atlas_man.atlases() {
            custom.game_model_renderer.flush_removal(&render_state.model_man, atlas.texture_id);
        }

        let changes = custom.build();
        for custom::RenderObjectChange {
            paint_id,
            old,
            new,
        } in changes
        {
            if let Some(new) = new {
                match new.ty {
                    custom::RenderObjectType::GameModel(new_ty) => {
                        if let Some(old) = old
                            && old.size == new.size
                            && let custom::RenderObjectType::GameModel(old_ty) = old.ty
                            && old_ty.model == new_ty.model
                        {
                            custom
                                .game_model_renderer
                                .modify(&render_state.model_man, new_ty.instance, paint_id, new.texture_id);
                        } else {
                            if let Some(old) = old {
                                custom.game_model_renderer.remove(paint_id, old.texture_id);
                                custom.game_model_renderer.flush_removal(&render_state.model_man, old.texture_id);
                            }

                            custom
                                .game_model_renderer
                                .insert(resource_man, &render_state.model_man, new_ty, paint_id, new.texture_id);
                        }
                    },
                }
            } else {
                // if `new` is none, then we're removing.
                // we assume `new` != `old`, so if `new` is None, then `old` must *not* be None.
                let old = old.unwrap();
                match old {
                    custom::RenderObject {
                        ty: custom::RenderObjectType::GameModel(..),
                        ..
                    } => {
                        custom.game_model_renderer.remove(paint_id, old.texture_id);
                    },
                }
            }
        }

        custom.game_model_renderer.render(
            &render.res.device,
            &render.res.queue,
            &render.res.global_res,
            encoder,
            render_state,
            &custom.atlas_man,
            render.frame_start,
        );
    }

    // --- yakui ---
    let vertices = gui.yakui_buffers.vertices.upload(&render.res.device, &render.res.queue);
    let indices = gui.yakui_buffers.indices.upload(&render.res.device, &render.res.queue);
    // --- yakui ---

    {
        // --- yakui ---
        let main_pipeline = yakui_wgpu::main_pipeline(&mut gui.yakui_wgpu.main_pipeline, &render.res.device, &surface);
        let text_pipeline = yakui_wgpu::text_pipeline(&mut gui.yakui_wgpu.text_pipeline, &render.res.device, &surface);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("yakui Render Pass"),
            color_attachments: &[Some(surface.color_attachment)],
            ..Default::default()
        });

        render_pass.set_vertex_buffer(0, vertices.slice(..));
        render_pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);

        let surface = paint.surface_size().as_uvec2();
        render_pass.set_viewport(0.0, 0.0, surface.x as f32, surface.y as f32, 0.0, 1.0);

        let mut last_clip = None;
        // --- yakui ---

        for (clip, draw_call) in draw_calls {
            // --- yakui ---
            if Some(clip) != last_clip {
                last_clip = Some(clip);

                let surface = paint.surface_size().as_uvec2();

                let pos = clip.pos().as_uvec2();
                let size = clip.size().as_uvec2();

                let max = (pos + size).min(surface);
                let size = UVec2::new(max.x.saturating_sub(pos.x), max.y.saturating_sub(pos.y));

                // If the scissor rect isn't valid, we can skip this
                // entire draw call.
                if pos.x > surface.x || pos.y > surface.y || size.x == 0 || size.y == 0 {
                    continue;
                }

                render_pass.set_scissor_rect(pos.x, pos.y, size.x, size.y);
            }
            // --- yakui ---

            match draw_call {
                DrawCall::Yakui(call) => {
                    YakuiWgpu::draw_yakui(
                        &gui.yakui_wgpu.texture_bindgroup_cache,
                        &mut render_pass,
                        main_pipeline,
                        text_pipeline,
                        call,
                    );
                },
                DrawCall::User(_) => {
                    unimplemented!();
                },
            }
        }
    }

    // cleanup for next frame
    paint
        .globals
        .get_mut()
        .get_mut(custom::CustomRenderer::default)
        .finish(&render.res, &render_state.model_man);
}
