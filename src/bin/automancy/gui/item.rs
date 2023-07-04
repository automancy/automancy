use egui::{vec2, Ui};

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

    let (_ui_id, rect) = ui.allocate_space(vec2(size, size));

    ui.label(format!(
        "{} ({})",
        resource_man.item_name(&stack.item.id),
        stack.amount
    ));

    let model = resource_man.get_item_model(stack.item);

    gui_instances.push((InstanceData::default(), model, Some(rect), None));
}
