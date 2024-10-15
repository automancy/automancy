use crate::GameState;
use automancy_defs::colors::BACKGROUND_3;
use automancy_ui::{col, label, movable, window, DIVIER_HEIGHT, DIVIER_THICKNESS};
use ron::ser::PrettyConfig;
use yakui::{divider, widgets::Layer};

/// Draws the debug menu (F3).
pub fn debugger(state: &mut GameState) {
    let fps = 1.0 / state.loop_store.elapsed.as_secs_f64();

    let reg_tiles = state.resource_man.registry.tiles.len();
    let reg_items = state.resource_man.registry.items.len();
    let tags = state.resource_man.registry.tags.len();
    let functions = state.resource_man.functions.len();
    let scripts = state.resource_man.registry.scripts.len();
    let audio = state.resource_man.audio.len();
    let meshes = state.resource_man.all_models.len();

    let Some((info, map_name)) = &state.loop_store.map_info else {
        return;
    };

    let map_info = state.tokio.block_on(info.lock()).clone();

    Layer::new().show(|| {
        let mut pos = state.ui_state.player_ui_position;
        movable(&mut pos, || {
            window(
                state.resource_man
                    .gui_str(state.resource_man.registry.gui_ids.debug_menu)
                    .to_string(),
                || {
                    col(|| {
                        label(&format!("FPS: {fps:.1}"));
                        label(&format!(
                            "WGPU: {}",
                            ron::ser::to_string_pretty(
                                &state.renderer.as_ref().unwrap().gpu.adapter_info,
                                PrettyConfig::default()
                            )
                            .unwrap_or("could not format wgpu info".to_string())
                        ));

                        divider(BACKGROUND_3, DIVIER_HEIGHT, DIVIER_THICKNESS);

                        label(&format!("ResourceMan: Tiles={reg_tiles} Items={reg_items} Tags={tags} Functions={functions} Scripts={scripts} Audio={audio} Meshes={meshes}"));

                        divider(BACKGROUND_3, DIVIER_HEIGHT, DIVIER_THICKNESS);

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
                }
            );
        });
        state.ui_state.player_ui_position = pos;
    });
}
