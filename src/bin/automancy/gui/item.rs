use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

use automancy::gpu;
use automancy::renderer::Renderer;
use automancy_defs::cg::Float;
use automancy_defs::egui::{vec2, Label, PaintCallback, Response, Sense, Ui, Widget};
use automancy_defs::egui_winit_vulkano::CallbackFn;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::id::Id;
use automancy_defs::rendering::{InstanceData, LightInfo};
use automancy_defs::vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use automancy_defs::vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage};
use automancy_resources::data::item::Item;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::ResourceManager;

fn paint_item(
    model: Id,
    resource_man: Arc<ResourceManager>,
    renderer: &Renderer,
) -> Arc<CallbackFn> {
    lazy_static! {
        static ref CALLBACKS: Arc<Mutex<HashMap<Id, Arc<CallbackFn>>>> =
            Arc::new(Mutex::new(HashMap::new()));
    }

    let pipeline = renderer.gpu.gui_pipeline.clone();
    let vertex_buffer = renderer.gpu.alloc.vertex_buffer.clone();
    let index_buffer = renderer.gpu.alloc.index_buffer.clone();

    CALLBACKS
        .lock()
        .unwrap()
        .entry(model)
        .or_insert_with(|| {
            Arc::new(CallbackFn::new(move |_info, context| {
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
                        .bind_vertex_buffers(
                            0,
                            (vertex_buffer.clone(), instance_buffer, light_info),
                        )
                        .bind_index_buffer(index_buffer.clone())
                        .draw_indexed_indirect(indirect_commands)
                        .unwrap();
                }
            }))
        })
        .clone()
}

/// Draws an Item's icon.
pub fn draw_item(
    ui: &mut Ui,
    resource_man: Arc<ResourceManager>,
    renderer: &Renderer,
    item: Item,
    size: Float,
) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::focusable_noninteractive());

    let callback = paint_item(resource_man.get_item_model(item), resource_man, renderer);

    ui.painter().add(PaintCallback { rect, callback });
}

pub struct ItemStackGuiElement {
    callback: Arc<CallbackFn>,
    label: Label,
}

impl ItemStackGuiElement {
    pub fn new(resource_man: Arc<ResourceManager>, renderer: &Renderer, stack: ItemStack) -> Self {
        let callback = paint_item(
            resource_man.get_item_model(stack.item),
            resource_man.clone(),
            renderer,
        );

        let label = Label::new(format!(
            "{} ({})",
            resource_man.item_name(&stack.item.id),
            stack.amount
        ));

        Self { callback, label }
    }
}

impl Widget for ItemStackGuiElement {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = ui.available_height();

        let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::focusable_noninteractive());

        ui.painter().add(PaintCallback {
            rect,
            callback: self.callback.clone(),
        });

        ui.add(self.label)
    }
}
