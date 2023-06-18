use crate::gui::default_frame;
use crate::setup::GameSetup;
use automancy::gpu;
use automancy::renderer::Renderer;
use automancy::tile_entity::TileModifier;
use automancy_defs::cg::{perspective, Matrix4, Vector3};
use automancy_defs::cgmath::{point3, vec3};
use automancy_defs::egui::{
    vec2, CursorIcon, Margin, PaintCallback, ScrollArea, Sense, TopBottomPanel, Ui,
};
use automancy_defs::egui_winit_vulkano::{CallbackFn, Gui};
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::Id;
use automancy_defs::rendering::{InstanceData, LightInfo};
use futures::channel::mpsc;
use std::f32::consts::FRAC_PI_4;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage};

/// Draws the tile selection.
fn paint_tile_selection(
    setup: &GameSetup,
    renderer: &Renderer,
    ui: &mut Ui,
    selected_tile_modifiers: &HashMap<Id, TileModifier>,
    mut selection_send: mpsc::Sender<Id>,
) {
    let size = ui.available_height();

    setup
        .resource_man
        .ordered_tiles
        .iter()
        .flat_map(|id| {
            setup
                .resource_man
                .registry
                .tile(*id)
                .unwrap()
                .models
                .get(*selected_tile_modifiers.get(id).unwrap_or(&0) as usize)
                .map(|model| (*id, *model))
        })
        .for_each(|(id, model)| {
            let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());

            response
                .clone()
                .on_hover_text(setup.resource_man.tile_name(&id));
            response.clone().on_hover_cursor(CursorIcon::Grab);

            let hover = if response.hovered() {
                ui.ctx()
                    .animate_value_with_time(ui.next_auto_id(), 1.0, 0.3)
            } else {
                ui.ctx()
                    .animate_value_with_time(ui.next_auto_id(), 0.0, 0.3)
            };
            if response.clicked() {
                selection_send.try_send(id).unwrap();
            }

            let pos = point3(0.0, 1.0 * hover + 0.5, 3.0 - 0.5 * hover);
            let matrix = perspective(FRAC_PI_4, 1.0, 0.01, 10.0)
                * Matrix4::look_to_rh(pos, vec3(0.0, 0.5 * hover + 0.2, 1.0), Vector3::unit_y());

            let pipeline = renderer.gpu.gui_pipeline.clone();
            let vertex_buffer = renderer.gpu.alloc.vertex_buffer.clone();
            let index_buffer = renderer.gpu.alloc.index_buffer.clone();
            let resource_man = setup.resource_man.clone();

            let callback = PaintCallback {
                rect,
                callback: Arc::new(CallbackFn::new(move |_info, context| {
                    let instance = (
                        InstanceData::default().with_model_matrix(matrix).into(),
                        model,
                    );

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
                            light_pos: [0.0, 0.0, 12.0],
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
                            .bind_vertex_buffers(
                                0,
                                (vertex_buffer.clone(), instance_buffer, light_info),
                            )
                            .bind_index_buffer(index_buffer.clone())
                            .draw_indexed_indirect(indirect_commands)
                            .unwrap();
                    }
                })),
            };

            ui.painter().add(callback);
        });
}

/// Creates the tile selection GUI.
pub fn tile_selections(
    setup: &GameSetup,
    renderer: &Renderer,
    gui: &Gui,
    selected_tile_modifiers: &HashMap<Id, TileModifier>,
    selection_send: mpsc::Sender<Id>,
) {
    TopBottomPanel::bottom("tile_selections")
        .show_separator_line(false)
        .resizable(false)
        .frame(default_frame().outer_margin(Margin::same(10.0)))
        .show(&gui.context(), |ui| {
            let spacing = ui.spacing_mut();

            spacing.interact_size.y = 70.0;
            spacing.scroll_bar_width = 0.0;
            spacing.scroll_bar_outer_margin = 0.0;

            ScrollArea::horizontal()
                .always_show_scroll(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        paint_tile_selection(
                            setup,
                            renderer,
                            ui,
                            selected_tile_modifiers,
                            selection_send,
                        );
                    });
                });
        });
}
