use yakui::flexible;

use crate::*;

/// Draws an Item's icon, with optional prefix and label text.
#[track_caller]
pub fn draw_item_stack_complex(resource_man: &ResourceManager, stack: ItemStack, size: f32, prefix: impl FnOnce(), add_label: bool) {
    row_cross_center(|| {
        prefix();

        GameModel::new(GenericModel::Item(stack.id), Vec2::splat(size)).show();

        if add_label {
            if stack.amount > 0 {
                Text::normal(format!("{} ({})", resource_man.item_name(stack.id), stack.amount)).show();
            } else {
                Text::normal(resource_man.item_name(stack.id).to_string()).show();
            }
        }
    });
}

#[track_caller]
pub fn draw_item(resource_man: &ResourceManager, id: ItemId, size: f32) {
    draw_item_stack_complex(
        resource_man,
        ItemStack {
            id,
            amount: 0,
        },
        size,
        || {},
        true,
    );
}

#[track_caller]
pub fn draw_item_stack(resource_man: &ResourceManager, stack: ItemStack, size: f32) {
    draw_item_stack_complex(resource_man, stack, size, || {}, true);
}

#[track_caller]
pub fn draw_recipe(resource_man: &ResourceManager, id: RecipeId, size: f32) {
    if let Some(stacks) = resource_man.registry.recipe_defs.get(&id).map(|recipe| &recipe.outputs) {
        for stack in stacks {
            draw_item_stack_complex(resource_man, *stack, size, || {}, false);
        }
    }

    Text::normal(resource_man.recipe_name(id).to_string()).show();
}

#[track_caller]
pub fn draw_recipe_info(game_state: &mut AutomancyGameState, id: RecipeId, size: f32) {
    if let Some(recipe) = game_state.resource_man.registry.recipe_defs.get(&id) {
        let has_inputs = recipe.inputs.is_some();

        if has_inputs {
            Text::normal(gui_str!(game_state, tile_config_recipe_inputs_and_outputs)).show();
        } else {
            Text::normal(gui_str!(game_state, tile_config_recipe_outputs)).show();
        }

        list_row()
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .item_spacing(sizing::PADDING_LARGE)
            .show(|| {
                if let Some(inputs) = &recipe.inputs {
                    flexible(1, || {
                        section(|| {
                            col(|| {
                                for &input in inputs {
                                    draw_item_stack(&game_state.resource_man, input, size);
                                }
                            });
                        });
                    });

                    Text::with_style("\u{f178}", TextStyle::symbols().font_size(sizing::HEADING_TEXT)).show();
                }

                flexible(1, || {
                    section(|| {
                        col(|| {
                            for &output in &recipe.outputs {
                                draw_item_stack(&game_state.resource_man, output, size);
                            }
                        });
                    });
                });
            });
    }
}
