use egui::{vec2, Sense, Ui};

use automancy::renderer::GuiInstances;
use automancy_defs::rendering::InstanceData;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::ResourceManager;

/// Draws an Item's icon.
pub fn draw_item(
    resource_man: &ResourceManager,
    ui: &mut Ui,
    gui_instances: &mut GuiInstances,
    stack: ItemStack,
) {
    let size = ui.available_height();

    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::focusable_noninteractive());

    ui.label(format!(
        "{} ({})",
        resource_man.item_name(&stack.item.id),
        stack.amount
    ));

    let model = resource_man.get_item_model(stack.item);

    gui_instances.push((InstanceData::default(), model, rect));
}
