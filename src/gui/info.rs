use egui::{vec2, Align2, Window};
use ractor::rpc::CallResult;

use automancy_defs::colors;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::Data;

use crate::gui::item::draw_item;
use crate::gui::SMALL_ICON_SIZE;
use crate::tile_entity::TileEntityMsg;
use crate::GameState;

/// Draws the info GUI.
pub fn info_ui(state: &mut GameState) {
    Window::new(
        state.resource_man.translates.gui[&state.resource_man.registry.gui_ids.info].as_str(),
    )
    .id("info".into())
    .anchor(Align2::RIGHT_TOP, vec2(-10.0, 10.0))
    .resizable(false)
    .default_width(300.0)
    .show(&state.gui.context.clone(), |ui| {
        ui.colored_label(colors::DARK_GRAY, state.camera.pointing_at.to_string());

        let Some((tile, entity)) = state.loop_store.pointing_cache.blocking_lock().clone() else {
            return;
        };

        ui.label(state.resource_man.tile_name(&tile));

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
                    ui,
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
    });
}
