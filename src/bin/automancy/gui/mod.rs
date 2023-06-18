use std::sync::Arc;

use automancy::gpu;
use automancy::gpu::Gpu;
use automancy::map::MapInfo;
use automancy_defs::cg::{DPoint2, Float};
use automancy_defs::cgmath::MetricSpace;
use automancy_defs::egui::epaint::Shadow;
use automancy_defs::egui::style::{WidgetVisuals, Widgets};
use automancy_defs::egui::FontFamily::{Monospace, Proportional};
use automancy_defs::egui::{
    vec2, Color32, FontData, FontDefinitions, FontId, Frame, PaintCallback, Rgba, Rounding,
    ScrollArea, Stroke, Style, TextStyle, Ui, Visuals,
};
use automancy_defs::egui_winit_vulkano::{CallbackFn, Gui, GuiConfig};
use automancy_defs::id::Id;
use automancy_defs::rendering::{GameVertex, InstanceData, LightInfo};
use automancy_defs::winit::event_loop::EventLoop;
use automancy_defs::{cgmath, colors};
use automancy_resources::data::item::Item;
use automancy_resources::ResourceManager;
use fuse_rust::Fuse;
use genmesh::{EmitTriangles, Quad};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::image::SampleCount::Sample4;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage};

use crate::IOSEVKA_FONT;
use automancy::renderer::Renderer;

pub mod debug;
pub mod error;
pub mod menu;
pub mod tile_config;
pub mod tile_info;
pub mod tile_selection;

/// The state of the main game GUI.
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum GuiState {
    MainMenu,
    MapLoad,
    Options,
    Ingame,
    Paused,
}

/// The state of popups (which are on top of the main GUI), if any should be displayed.
#[derive(Clone)]
pub enum PopupState {
    None,
    MapCreate,
    MapDeleteConfirmation(MapInfo),
}

/// Initialize the font families.
fn init_fonts(gui: &Gui) {
    let mut fonts = FontDefinitions::default();
    let iosevka = "iosevka";

    fonts
        .font_data
        .insert(iosevka.to_owned(), FontData::from_static(IOSEVKA_FONT));

    fonts
        .families
        .get_mut(&Proportional)
        .unwrap()
        .insert(0, iosevka.to_owned());
    fonts
        .families
        .get_mut(&Monospace)
        .unwrap()
        .insert(0, iosevka.to_owned());

    gui.context().set_fonts(fonts);
}

/// Initialize the GUI style.
fn init_styles(gui: &Gui) {
    gui.context().set_style(Style {
        override_text_style: None,
        override_font_id: None,
        text_styles: [
            (TextStyle::Small, FontId::new(9.0, Proportional)),
            (TextStyle::Body, FontId::new(13.0, Proportional)),
            (TextStyle::Button, FontId::new(13.0, Proportional)),
            (TextStyle::Heading, FontId::new(19.0, Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, Monospace)),
        ]
        .into(),
        wrap: None,
        visuals: Visuals {
            widgets: Widgets {
                noninteractive: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(248),
                    bg_fill: Color32::from_gray(170),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(160)), // separators, indentation lines
                    fg_stroke: Stroke::new(1.0, Color32::from_gray(80)),  // normal text color
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
                inactive: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(200), // button background
                    bg_fill: Color32::from_gray(200),      // checkbox background
                    bg_stroke: Default::default(),
                    fg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // button text
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
                hovered: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(220),
                    bg_fill: Color32::from_gray(190),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(105)), // e.g. hover over window edge or button
                    fg_stroke: Stroke::new(1.5, Color32::BLACK),
                    rounding: Rounding::same(3.0),
                    expansion: 1.0,
                },
                active: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(165),
                    bg_fill: Color32::from_gray(180),
                    bg_stroke: Stroke::new(1.0, Color32::BLACK),
                    fg_stroke: Stroke::new(2.0, Color32::BLACK),
                    rounding: Rounding::same(2.0),
                    expansion: 1.0,
                },
                open: WidgetVisuals {
                    weak_bg_fill: Color32::from_gray(220),
                    bg_fill: Color32::from_gray(210),
                    bg_stroke: Stroke::new(1.0, Color32::from_gray(160)),
                    fg_stroke: Stroke::new(1.0, Color32::BLACK),
                    rounding: Rounding::same(2.0),
                    expansion: 0.0,
                },
            },
            ..Visuals::light()
        },
        ..Default::default()
    });
}

/// Initializes the GUI.
pub fn init_gui(event_loop: &EventLoop<()>, gpu: &Gpu) -> Gui {
    let gui = Gui::new_with_subpass(
        event_loop,
        gpu.surface.clone(),
        gpu.queue.clone(),
        gpu.gui_subpass.clone(),
        GuiConfig {
            preferred_format: Some(gpu.alloc.swapchain.image_format()),
            is_overlay: true,
            samples: Sample4,
        },
    );

    init_fonts(&gui);
    init_styles(&gui);

    gui
}

/// Creates a default frame.
pub fn default_frame() -> Frame {
    Frame::none()
        .fill(colors::WHITE.multiply(0.65).into())
        .shadow(Shadow {
            extrusion: 8.0,
            color: colors::DARK_GRAY.multiply(0.5).into(),
        })
        .rounding(Rounding::same(5.0))
}

/// Draws an Item's icon.
pub fn draw_item(
    ui: &mut Ui,
    resource_man: Arc<ResourceManager>,
    renderer: &Renderer,
    item: Item,
    size: Float,
) {
    let model = if resource_man.meshes.contains_key(&item.model) {
        item.model
    } else {
        resource_man.registry.model_ids.items_missing
    };

    let (_, rect) = ui.allocate_space(vec2(size, size));

    let pipeline = renderer.gpu.gui_pipeline.clone();
    let vertex_buffer = renderer.gpu.alloc.vertex_buffer.clone();
    let index_buffer = renderer.gpu.alloc.index_buffer.clone();

    let callback = PaintCallback {
        rect,
        callback: Arc::new(CallbackFn::new(move |_info, context| {
            let instance = (InstanceData::default().into(), model);

            let light_info = Buffer::from_data(
                &context.resources.memory_allocator,
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    usage: MemoryUsage::Upload,
                    ..Default::default()
                },
                LightInfo {
                    light_pos: [0.0, 0.0, 2.0],
                    light_color: [1.0; 4],
                },
            )
            .unwrap();

            if let Some((indirect_commands, instance_buffer)) = gpu::indirect_instance(
                &context.resources.memory_allocator,
                &resource_man,
                &[instance],
            ) {
                context
                    .builder
                    .bind_pipeline_graphics(pipeline.clone())
                    .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer, light_info))
                    .bind_index_buffer(index_buffer.clone())
                    .draw_indexed_indirect(indirect_commands)
                    .unwrap();
            }
        })),
    };

    ui.painter().add(callback);
}

/// Produces a line shape.
pub fn make_line(a: DPoint2, b: DPoint2, color: Rgba) -> Vec<GameVertex> {
    let v = b - a;
    let l = a.distance(b) * 128.0;
    let w = cgmath::vec2(-v.y / l, v.x / l);

    let a0 = (a + w).cast::<Float>().unwrap();
    let a1 = (b + w).cast::<Float>().unwrap();
    let b0 = (b - w).cast::<Float>().unwrap();
    let b1 = (a - w).cast::<Float>().unwrap();

    let mut line = vec![];

    Quad::new(
        GameVertex {
            pos: [a0.x, a0.y, 0.0],
            color: color.to_array(),
            normal: [0.0, 0.0, 0.0],
        },
        GameVertex {
            pos: [a1.x, a1.y, 0.0],
            color: color.to_array(),
            normal: [0.0, 0.0, 0.0],
        },
        GameVertex {
            pos: [b0.x, b0.y, 0.0],
            color: color.to_array(),
            normal: [0.0, 0.0, 0.0],
        },
        GameVertex {
            pos: [b1.x, b1.y, 0.0],
            color: color.to_array(),
            normal: [0.0, 0.0, 0.0],
        },
    )
    .emit_triangles(|v| line.append(&mut vec![v.x, v.y, v.z]));

    line
}

/// Draws a search bar.
pub fn searchable_id<'a>(
    ui: &mut Ui,
    resource_man: &'a ResourceManager,
    fuse: &Fuse,
    ids: &[Id],
    new_id: &mut Option<Id>,
    filter: &mut String,
    name: &'static impl Fn(&'a ResourceManager, &Id) -> &'a str,
) {
    ui.text_edit_singleline(filter);

    ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
        ui.set_width(ui.available_width());

        let ids = if !filter.is_empty() {
            let mut filtered = ids
                .iter()
                .flat_map(|id| {
                    let result = fuse.search_text_in_string(filter, name(resource_man, id));
                    let score = result.map(|v| v.score);

                    if score.unwrap_or(0.0) > 0.4 {
                        None
                    } else {
                        Some(*id).zip(score)
                    }
                })
                .collect::<Vec<_>>();

            filtered.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

            filtered.into_iter().map(|v| v.0).collect::<Vec<_>>()
        } else {
            ids.to_vec()
        };

        ids.iter().for_each(|script| {
            ui.radio_value(new_id, Some(*script), name(resource_man, script));
        })
    });
}
