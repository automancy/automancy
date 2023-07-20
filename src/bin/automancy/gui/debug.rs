use egui::{Context, Window};
use ractor::ActorRef;
use tokio::runtime::Runtime;

use automancy::game::GameMsg;

use crate::event::EventLoopStorage;
use crate::gui::default_frame;
use crate::setup::GameSetup;

/// Draws the debug menu (F3).
pub fn debugger(
    setup: &GameSetup,
    context: &Context,
    runtime: &Runtime,
    game: ActorRef<GameMsg>,
    loop_store: &mut EventLoopStorage,
) {
    let resource_man = setup.resource_man.clone();

    let fps = 1.0 / loop_store.elapsed.as_secs_f64();

    let reg_tiles = resource_man.registry.tiles.len();
    let reg_items = resource_man.registry.items.len();
    let tags = resource_man.registry.tags.len();
    //let functions = resource_man.functions.len();
    let scripts = resource_man.registry.scripts.len();
    let audio = resource_man.audio.len();
    let meshes = resource_man.meshes.len();

    let (info, map_name) = runtime
        .block_on(game.call(GameMsg::GetMapInfo, Some(loop_store.elapsed)))
        .unwrap()
        .unwrap();

    let tile_count = info.tile_count;

    Window::new(
        setup.resource_man.translates.gui[&resource_man.registry.gui_ids.debug_menu].as_str(),
    )
    .resizable(false)
    .default_width(600.0)
    .frame(default_frame())
    .show(context, |ui| {
        ui.label(format!("FPS: {fps:.1}"));
        //ui.label(format!("Device: {device_name} API {api_version}"));
        ui.label(format!(
            "ResourceMan: {reg_tiles}T {reg_items}I {tags}Ta {scripts}S {audio}A {meshes}M"
        ));
        ui.label(format!(
            "Map \"{map_name}\" ({map_name}.run): {tile_count}T"
        ))
    });
}
