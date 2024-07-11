use automancy_defs::math::Float;
use automancy_defs::rendering::InstanceData;
use automancy_defs::{glam::vec2, stack::ItemStack};
use automancy_resources::{types::IconMode, ResourceManager};
use yakui::Rect;

use super::{center_row, label, ui_game_object};

/// Draws an Item's icon.
pub fn draw_item(
    resource_man: &ResourceManager,
    prefix: impl FnOnce(),
    stack: ItemStack,
    size: Float,
    add_label: bool,
) -> Option<Rect> {
    let mut rect = None;

    center_row(|| {
        prefix();

        rect = ui_game_object(
            InstanceData::default(),
            resource_man.item_model_or_missing(stack.id),
            vec2(size, size),
            Some(IconMode::Item.world_matrix()),
        )
        .into_inner();

        if add_label {
            if stack.amount > 0 {
                label(&format!(
                    "{} ({})",
                    resource_man.item_name(stack.id),
                    stack.amount
                ));
            } else {
                label(&resource_man.item_name(stack.id));
            }
        }
    });

    rect
}
