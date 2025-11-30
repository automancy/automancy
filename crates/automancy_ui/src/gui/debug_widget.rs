use automancy_data::game::generic::serailize::IdMapping;
use automancy_game::resources::global::debug_id;

use crate::*;

/// Draws the debug widget (F3).
#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn debug_widget(ctx: &mut UiContext) {
    let mut closed = false;

    if !ctx.game_state.input_handler.key_active(ActionType::Debug) {
        ctx.gui.state.debugger_ui_state.clear();
        closed = true
    }

    let tile_count = ctx.game_state.resource_man.registry.tile_defs.len();
    let item_count = ctx.game_state.resource_man.registry.item_defs.len();
    let tag_count = ctx.game_state.resource_man.registry.tag_defs.len();
    let recipe_count = ctx.game_state.resource_man.registry.recipe_defs.len();
    let script_count = ctx.game_state.resource_man.scripts.len();
    let audio_count = ctx.game_state.resource_man.audio.len();

    let fps = (1.0 / ctx.render.frame_time.as_secs_f32()).round();

    let vertex_count = ctx.render_state.model_man.vertices.len();
    let index_count = ctx.render_state.model_man.indices.len();
    let mesh_count = ctx.render_state.model_man.mesh_count();
    let animation_count = ctx.render_state.model_man.animation_count();

    let resource_man_stats = format!(
        "ResourceMan: Tiles={tile_count} Items={item_count} Tags={tag_count} Recipes={recipe_count} Scripts={script_count} Audio={audio_count}"
    );
    let model_man_stats =
        format!("ModelMan: Meshes={mesh_count} Vertices={vertex_count} Indices={index_count} Animations={animation_count}");

    ctx.gui.state.debugger_ui_state =
        Window::new(gui_str!(ctx.game_state, debug_menu_title))
            .closed(closed)
            .show_movable(ctx.gui.state.debugger_ui_state, || {
                col(|| {
                    Text::normal(format!("FPS: {fps:}")).show();
                    Text::normal(format!(
                        "WGPU: {}",
                        persistent::ron::to_string_pretty(&ctx.render.res.adapter_info).unwrap_or("could not format wgpu info".to_string())
                    ))
                    .show();

                    divider(colors::BACKGROUND_3.yak(), sizing::DIVIER_HEIGHT, sizing::DIVIER_THICKNESS);

                    Text::normal(resource_man_stats).show();

                    Text::normal(model_man_stats).show();

                    divider(colors::BACKGROUND_3.yak(), sizing::DIVIER_HEIGHT, sizing::DIVIER_THICKNESS);

                    if let Some((map_id, map_info)) = ctx.game_data.loaded_map.read_latest() {
                        let map_data = map_info
                            .data
                            .iter()
                            .map(|(key, datum)| {
                                format!(
                                    "\"{}\": {}",
                                    debug_id(*key),
                                    persistent::ron::to_string_pretty(
                                        &datum.clone().into_raw(&mut IdMapping::new(), &ctx.game_state.resource_man.interner)
                                    )
                                    .unwrap_or("could not format map data".to_string()),
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(",\n");
                        let map_data = format!("\n{map_data}\n");

                        Text::normal(format!("Map \"{map_id}\"",)).show();
                        Text::normal(format!("Save time: {:?}", map_info.mtime)).show();
                        Text::normal(format!("Data: {map_data}")).show();
                    }
                });
            });
}
