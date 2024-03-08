use std::time::SystemTime;

use egui::{Context, Window};
use ron::ser::PrettyConfig;
use tokio::runtime::Runtime;

use crate::event::EventLoopStorage;
use crate::renderer::Renderer;
use crate::setup::GameSetup;

/// Draws the debug menu (F3).
pub fn debugger(
    runtime: &Runtime,
    setup: &GameSetup,
    loop_store: &mut EventLoopStorage,
    renderer: &Renderer,
    context: &Context,
) {
    let resource_man = setup.resource_man.clone();

    let fps = 1.0 / loop_store.elapsed.as_secs_f64();

    let reg_tiles = resource_man.registry.tiles.len();
    let reg_items = resource_man.registry.items.len();
    let tags = resource_man.registry.tags.len();
    let functions = resource_man.functions.len();
    let scripts = resource_man.registry.scripts.len();
    let audio = resource_man.audio.len();
    let meshes = resource_man.all_models.len();

    let Some((info, map_name)) = &loop_store.map_info else {
        return;
    };

    let map_info = runtime.block_on(info.lock()).clone();

    Window::new(
        setup.resource_man.translates.gui[&resource_man.registry.gui_ids.debug_menu].as_str(),
    )
    .resizable(false)
    .default_width(600.0)
    .show(context, |ui| {
        ui.label(format!("FPS: {fps:.1}"));
        ui.label(format!("WGPU: {}", ron::ser::to_string_pretty(&renderer.gpu.adapter_info, PrettyConfig::default()).unwrap_or("could not format wgpu info".to_string())));
        ui.separator();
        ui.label(format!(
            "ResourceMan: Tiles={reg_tiles} Items={reg_items} Tags={tags} Functions={functions} Scripts={scripts} Audio={audio} Meshes={meshes}"
        ));
        ui.label(format!("Map \"{map_name}\" ({:?}): {}", map_info.save_time.unwrap_or(SystemTime::UNIX_EPOCH), ron::ser::to_string_pretty(&map_info.data.to_raw(&setup.resource_man.interner), PrettyConfig::default()).unwrap_or("could not format map info".to_string())));
    });
}
