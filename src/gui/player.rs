use egui::{ScrollArea, Window};

use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};

use crate::gui::item::draw_item;
use crate::gui::{take_item_animation, MEDIUM_ICON_SIZE};
use crate::GameState;

pub fn player(state: &mut GameState, game_data: &mut DataMap) {
    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.player_menu]
            .as_str(),
    )
    .resizable(false)
    .collapsible(false)
    .show(&state.gui.context.clone(), |ui| {
        ui.label(
            state.resource_man.translates.gui
                [&state.resource_man.registry.gui_ids.player_inventory]
                .as_str(),
        );

        if let Some(Data::Inventory(inventory)) =
            game_data.get(&state.resource_man.registry.data_ids.player_inventory)
        {
            ScrollArea::vertical().show(ui, |ui| {
                for (id, amount) in inventory.iter() {
                    if let Some(item) = state.resource_man.registry.items.get(id) {
                        let (dst_rect, _) = draw_item(
                            &state.resource_man,
                            ui,
                            None,
                            ItemStack {
                                item: *item,
                                amount: *amount,
                            },
                            MEDIUM_ICON_SIZE,
                            true,
                        );

                        take_item_animation(state, ui, *item, dst_rect);
                    }
                }
            });
        }

        if ui
            .button(
                state.resource_man.translates.gui
                    [&state.resource_man.registry.gui_ids.open_research]
                    .as_str(),
            )
            .clicked()
        {
            state.gui_state.research_open = true;
        }
    });
}
