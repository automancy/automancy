use std::time::SystemTime;

use egui::Window;
use ron::ser::PrettyConfig;

use crate::GameState;

/// Draws the debug menu (F3).
pub fn debugger(state: &GameState) {
    let resource_man = &*state.resource_man;

    let fps = 1.0 / state.loop_store.elapsed.as_secs_f64();

    let reg_tiles = resource_man.registry.tiles.len();
    let reg_items = resource_man.registry.items.len();
    let tags = resource_man.registry.tags.len();
    let functions = resource_man.functions.len();
    let scripts = resource_man.registry.scripts.len();
    let audio = resource_man.audio.len();
    let meshes = resource_man.all_models.len();

    let Some((info, map_name)) = &state.loop_store.map_info else {
        return;
    };

    let map_info = state.tokio.block_on(info.lock()).clone();

    Window::new(
        resource_man.translates.gui[&resource_man.registry.gui_ids.debug_menu].as_str(),
    )
        .resizable(false)
        .default_width(600.0)
        .show(&state.gui.context.clone(), |ui| {
            ui.label(format!("FPS: {fps:.1}"));
            ui.label(format!("WGPU: {}", ron::ser::to_string_pretty(&state.renderer.gpu.adapter_info, PrettyConfig::default()).unwrap_or("could not format wgpu info".to_string())));
            ui.separator();
            ui.label(format!(
                "ResourceMan: Tiles={reg_tiles} Items={reg_items} Tags={tags} Functions={functions} Scripts={scripts} Audio={audio} Meshes={meshes}"
            ));
            ui.label(format!("Map \"{map_name}\" ({:?}): {}", map_info.save_time.unwrap_or(SystemTime::UNIX_EPOCH), ron::ser::to_string_pretty(&map_info.data.to_raw(&state.resource_man.interner), PrettyConfig::default()).unwrap_or("could not format map info".to_string())));
        });
}
