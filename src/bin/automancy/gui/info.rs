use egui::{vec2, Align2, Context, Window};
use futures::executor::block_on;

use automancy::game::GameMsg;
use automancy::tile_entity::TileEntityMsg;
use automancy_defs::colors;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::Data;

use crate::gui::default_frame;
use crate::gui::item::{draw_item, SMALL_ITEM_ICON_SIZE};
use crate::renderer::GuiInstances;
use crate::setup::GameSetup;

/// Draws the info GUI.
pub fn info(setup: &GameSetup, item_instances: &mut GuiInstances, context: &Context) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.info].as_str(),
    )
    .anchor(Align2::RIGHT_TOP, vec2(-10.0, 10.0))
    .resizable(false)
    .default_width(300.0)
    .frame(default_frame())
    .show(context, |ui| {
        ui.colored_label(colors::DARK_GRAY, setup.camera.pointing_at.to_string());

        let tile_entity = block_on(setup.game.call(
            |reply| GameMsg::GetTileEntity(setup.camera.pointing_at, reply),
            None,
        ))
        .unwrap()
        .unwrap();

        let tile = block_on(setup.game.call(
            |reply| GameMsg::GetTile(setup.camera.pointing_at, reply),
            None,
        ))
        .unwrap()
        .unwrap();

        if let Some((tile_entity, (id, _))) = tile_entity.zip(tile) {
            ui.label(setup.resource_man.tile_name(&id));

            let data = block_on(tile_entity.call(TileEntityMsg::GetData, None))
                .unwrap()
                .unwrap();

            if let Some(inventory) = data
                .get(&setup.resource_man.registry.data_ids.buffer)
                .and_then(Data::as_inventory)
                .cloned()
            {
                for (item, amount) in inventory.iter().flat_map(|(id, amount)| {
                    setup
                        .resource_man
                        .registry
                        .item(*id)
                        .map(|item| (*item, *amount))
                }) {
                    draw_item(
                        &setup.resource_man,
                        ui,
                        item_instances,
                        None,
                        ItemStack { item, amount },
                        SMALL_ITEM_ICON_SIZE,
                    );
                }
            }
            //ui.label(format!("State: {}", ask(sys, &game, )))
        }
    });
}
