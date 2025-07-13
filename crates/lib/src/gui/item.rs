use automancy_defs::{
    math::{Float, vec2},
    rendering::InstanceData,
    stack::ItemStack,
};
use automancy_resources::{ResourceManager, types::IconMode};
use automancy_ui::{UiGameObjectType, center_row, label, ui_game_object};

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
            InstanceData::default(),
            UiGameObjectType::Model(resource_man.item_model_or_missing(&stack.id)),
            vec2(size, size),
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
