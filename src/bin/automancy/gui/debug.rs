use ractor::ActorRef;
use tokio::runtime::Runtime;

use automancy::game::GameMsg;
use automancy::renderer::Renderer;
use automancy_defs::egui::{Margin, Window};
use automancy_defs::egui_winit_vulkano::Gui;

use crate::event::EventLoopStorage;
use crate::gui::default_frame;
use crate::setup::GameSetup;

/// Draws the debug menu (F3).
pub fn debugger(
    setup: &GameSetup,
    gui: &mut Gui,
    runtime: &Runtime,
    game: ActorRef<GameMsg>,
    renderer: &Renderer,
    loop_store: &mut EventLoopStorage,
) {
    let resource_man = setup.resource_man.clone();
    let device_name = renderer
        .gpu
        .alloc
        .physical_device
        .properties()
        .device_name
        .clone();
    let api_version = renderer
        .gpu
        .surface
        .instance()
        .max_api_version()
        .to_string();

    let fps = 1.0 / loop_store.elapsed.as_secs_f64();

    let reg_tiles = resource_man.registry.tiles.len();
    let reg_items = resource_man.registry.items.len();
    let tags = resource_man.registry.tags.len();
    //let functions = resource_man.functions.len();
    let scripts = resource_man.registry.scripts.len();
    let audio = resource_man.audio.len();
    let meshes = resource_man.meshes.len();

    let map = runtime
        .block_on(game.call(GameMsg::GetMapInfo, Some(loop_store.elapsed)))
        .unwrap()
        .unwrap();

    let map_name = map.map_name;
    let data_size = map.data;
    let tile_count = map.tiles;

    Window::new(
        setup.resource_man.translates.gui[&resource_man.registry.gui_ids.debug_menu].as_str(),
    )
    .resizable(false)
    .default_width(600.0)
    .frame(default_frame().inner_margin(Margin::same(10.0)))
    .show(&gui.context(), |ui| {
        ui.label(format!("FPS: {fps:.1}"));
        ui.label(format!("Device: {device_name} API {api_version}"));
        ui.label(format!(
            "ResourceMan: {reg_tiles}T {reg_items}I {tags}Ta {scripts}S {audio}A {meshes}M"
        ));
        ui.label(format!(
            "Map \"{map_name}\" ({map_name}.run): {data_size}D {tile_count}T"
        ))
    });
}