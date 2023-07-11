use egui::{vec2, Sense, Ui};

use automancy::renderer::GuiInstances;
use automancy_defs::rendering::InstanceData;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::ResourceManager;

use crate::gui::ITEM_ICON_SIZE;

/// Draws an Item's icon.
pub fn draw_item(
    resource_man: &ResourceManager,
    ui: &mut Ui,
    gui_instances: &mut GuiInstances,
    prefix: Option<&'static str>,
    stack: ItemStack,
) {
    ui.horizontal(|ui| {
        ui.set_height(ITEM_ICON_SIZE);
        ui.style_mut().spacing.item_spacing = vec2(0.0, 0.0);

        let size = ui.available_height();

        if let Some(prefix) = prefix {
            ui.label(prefix);
        }

        let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());

        if stack.amount > 0 {
            ui.label(format!(
                "{} ({})",
                resource_man.item_name(&stack.item.id),
                stack.amount
            ));
        } else {
            ui.label(resource_man.item_name(&stack.item.id).to_string());
        }

        let model = resource_man.get_item_model(stack.item);

        gui_instances.push((InstanceData::default(), model, Some(rect), None));
    });
}
