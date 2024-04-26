use automancy_defs::colors::INACTIVE;
use ron::ser::PrettyConfig;
use yakui::{column, divider, use_state, Vec2};

use crate::GameState;

use super::{label, movable, window, DIVIER_SIZE};

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

    let pos_state = use_state(|| Vec2::ZERO);

    let mut pos = pos_state.get();
    movable(&mut pos, || {
        window(
            resource_man.translates.gui[&resource_man.registry.gui_ids.debug_menu].to_string(),
            || {
                column(|| {
                    label(&format!("FPS: {fps:.1}"));
                    label(&format!(
                        "WGPU: {}",
                        ron::ser::to_string_pretty(
                            &state.renderer.gpu.adapter_info,
                            PrettyConfig::default()
                        )
                        .unwrap_or("could not format wgpu info".to_string())
                    ));

                    divider(INACTIVE, DIVIER_SIZE, DIVIER_SIZE);

                    label(&format!("ResourceMan: Tiles={reg_tiles} Items={reg_items} Tags={tags} Functions={functions} Scripts={scripts} Audio={audio} Meshes={meshes}"));

                    divider(INACTIVE, DIVIER_SIZE, DIVIER_SIZE);

                    label(&format!("Map \"{map_name}\"",));
                    label(&format!("Save Time: {:?}", &map_info.save_time));
                    label(&format!(
                        "Info: {}",
                        &ron::ser::to_string_pretty(
                            &map_info.data.to_raw(&state.resource_man.interner),
                            PrettyConfig::default(),
                        )
                        .unwrap_or("could not format map info".to_string()),
                    ));
                });
            },
        );
    });
    pos_state.set(pos);
}
