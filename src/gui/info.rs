use ractor::rpc::CallResult;

use automancy_defs::colors;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::Data;
use yakui::{align, Alignment};

use crate::gui::item::draw_item;
use crate::gui::SMALL_ICON_SIZE;
use crate::tile_entity::TileEntityMsg;
use crate::GameState;

use super::components::{
    text::{colored_label, label},
    window::window,
};

/// Draws the info GUI.
pub fn info_ui(state: &mut GameState) {
    align(Alignment::TOP_RIGHT, || {
        window(
            state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.info]
                .to_string(),
            || {
                colored_label(&state.camera.pointing_at.to_string(), colors::DARK_GRAY);

                let Some((tile, entity)) = state.loop_store.pointing_cache.blocking_lock().clone()
                else {
                    return;
                };

                label(&state.resource_man.tile_name(&tile));

                let Ok(CallResult::Success(data)) = state
                    .tokio
                    .block_on(entity.call(TileEntityMsg::GetData, None))
                else {
                    return;
                };

                if let Some(Data::Inventory(inventory)) =
                    data.get(&state.resource_man.registry.data_ids.buffer)
                {
                    for (id, amount) in inventory.iter() {
                        let item = state.resource_man.registry.items.get(id).unwrap();

                        draw_item(
                            &state.resource_man,
                            None,
                            ItemStack {
                                item: *item,
                                amount: *amount,
                            },
                            SMALL_ICON_SIZE,
                            true,
                        );
                    }
                }
            },
        );
    });
}
