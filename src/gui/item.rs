use automancy_defs::glam::{dvec3, vec2};
use automancy_defs::math;
use automancy_defs::math::Float;
use automancy_defs::rendering::InstanceData;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::ResourceManager;

use crate::gui::GameElement;

use super::components::{list::row, text::label};

/// Draws an Item's icon.
pub fn draw_item(
    resource_man: &ResourceManager,
    prefix: Option<&'static str>,
    stack: ItemStack,
    size: Float,
    add_label: bool,
) {
    row(|| {
        if let Some(prefix) = prefix {
            label(prefix);
        }

        GameElement::new(
            InstanceData::default().with_world_matrix(math::view(dvec3(0.0, 0.0, 1.0)).as_mat4()),
            resource_man.get_item_model(stack.item.model),
            vec2(size, size),
        )
        .show();

        if add_label {
            if stack.amount > 0 {
                label(&format!(
                    "{} ({})",
                    resource_man.item_name(&stack.item.id),
                    stack.amount
                ));
            } else {
                label(&resource_man.item_name(&stack.item.id));
            }
        }
    });
}
