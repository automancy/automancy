use tokio::runtime::Runtime;

use automancy::game::GameMsg;
use automancy::renderer::Renderer;
use automancy::tile_entity::TileEntityMsg;
use automancy_defs::colors;
use automancy_defs::egui::{vec2, Align, Align2, Margin, Window};
use automancy_defs::egui_winit_vulkano::Gui;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::Data;

use crate::gui::item::ItemStackGuiElement;
use crate::gui::{default_frame, ITEM_ICON_SIZE};
use crate::setup::GameSetup;

/// Draws the tile info GUI.
pub fn tile_info(runtime: &Runtime, setup: &GameSetup, renderer: &Renderer, gui: &Gui) {
    Window::new(
        setup.resource_man.translates.gui[&setup.resource_man.registry.gui_ids.tile_info]
            .to_string(),
    )
    .anchor(Align2([Align::RIGHT, Align::TOP]), vec2(-10.0, 10.0))
    .resizable(false)
    .default_width(300.0)
    .frame(default_frame().inner_margin(Margin::same(10.0)))
    .show(&gui.context(), |ui| {
        ui.colored_label(colors::DARK_GRAY, setup.camera.pointing_at.to_string());

        let tile_entity = runtime
            .block_on(setup.game.call(
                |reply| GameMsg::GetTileEntity(setup.camera.pointing_at, reply),
                None,
            ))
            .unwrap()
            .unwrap();

        let tile = runtime
            .block_on(setup.game.call(
                |reply| GameMsg::GetTile(setup.camera.pointing_at, reply),
                None,
            ))
            .unwrap()
            .unwrap();

        if let Some((tile_entity, (id, _))) = tile_entity.zip(tile) {
            ui.label(setup.resource_man.tile_name(&id));

            let data = runtime
                .block_on(tile_entity.call(TileEntityMsg::GetData, None))
                .unwrap()
                .unwrap();

            if let Some(inventory) = data
                .get(&setup.resource_man.registry.data_ids.buffer)
                .and_then(Data::as_inventory)
                .cloned()
            {
                for (item, amount) in inventory.0.into_iter() {
                    ui.horizontal(|ui| {
                        ui.set_height(ITEM_ICON_SIZE);
                        ui.add(ItemStackGuiElement::new(
                            setup.resource_man.clone(),
                            renderer,
                            ItemStack { item, amount },
                        ));
                    });
                }
            }
            //ui.label(format!("State: {}", ask(sys, &game, )))
        }
    });
}
