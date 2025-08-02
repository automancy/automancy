use automancy_data::{
    math::{Float, vec2},
    rendering::Instance,
    stack::ItemStack,
};
use automancy_resources::{ResourceManager, types::IconMode};
use automancy_ui::{GameObjectType, center_row, label, ui_game_object};

/// Draws an Item's icon.
pub fn draw_item(
    resource_man: &ResourceManager,
    prefix: impl FnOnce(),
    stack: ItemStack,
    size: Float,
    add_label: bool,
) {
    center_row(|| {
        prefix();

        ui_game_object(
            Instance::default(),
            GameObjectType::Model(resource_man.item_model_or_missing(&stack.id)),
            Vec2::new(size, size),
            Some(IconMode::Item.model_matrix()),
            Some(IconMode::Item.world_matrix()),
        );

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
}
