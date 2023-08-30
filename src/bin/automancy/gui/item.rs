use egui::{vec2, Rect, Response, Sense, Ui};

use automancy_defs::math::Float;
use automancy_defs::rendering::InstanceData;
use automancy_resources::data::item::Item;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::ResourceManager;

use crate::renderer::GuiInstances;

pub const SMALL_ITEM_ICON_SIZE: Float = 24.0;
pub const MEDIUM_ITEM_ICON_SIZE: Float = 48.0;
pub const LARGE_ITEM_ICON_SIZE: Float = 96.0;

pub fn paint_item(
    resource_man: &ResourceManager,
    item_instances: &mut GuiInstances,
    item: Item,
    rect: Rect,
) {
    let model = resource_man.get_item_model(item);

    item_instances.push((InstanceData::default(), model, (Some(rect), None)))
}

/// Draws an Item's icon.
pub fn draw_item(
    resource_man: &ResourceManager,
    ui: &mut Ui,
    item_instances: &mut GuiInstances,
    prefix: Option<&'static str>,
    stack: ItemStack,
    size: Float,
) -> (Rect, Response) {
    ui.horizontal(|ui| {
        ui.set_height(size);

        ui.style_mut().spacing.item_spacing = vec2(10.0, 0.0);

        if let Some(prefix) = prefix {
            ui.label(prefix);
        }

        let (rect, icon_response) = ui.allocate_exact_size(vec2(size, size), Sense::click());

        let label_response = if stack.amount > 0 {
            ui.label(format!(
                "{} ({})",
                resource_man.item_name(&stack.item.id),
                stack.amount
            ))
        } else {
            ui.label(resource_man.item_name(&stack.item.id).to_string())
        };

        paint_item(resource_man, item_instances, stack.item, rect);

        (rect, icon_response.union(label_response))
    })
    .inner
}
