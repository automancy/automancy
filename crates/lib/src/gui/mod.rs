use crate::GameState;
use automancy_defs::glam::vec3;
use automancy_defs::id::ModelId;
use automancy_defs::rendering::InstanceData;
use automancy_defs::{colors, math, rendering::make_line, window};
use automancy_defs::{
    math::{Float, Matrix4, FAR, HEX_GRID_LAYOUT},
    rendering::GameMatrix,
};
use automancy_resources::data::DataMap;
use automancy_system::input::ActionType;
use automancy_system::ui_state::{PopupState, Screen};
use tokio::sync::oneshot;
use util::render_overlay_cached;
use winit::event_loop::ActiveEventLoop;

pub mod debug;
pub mod error;
pub mod info;
pub mod item;
pub mod menu;
pub mod player;
pub mod popup;
pub mod tile_config;
pub mod tile_selection;
pub mod util;

pub fn render_ui(
    state: &mut GameState,
    result: &mut anyhow::Result<bool>,
    event_loop: &ActiveEventLoop,
) {
    if state.ui_state.popup == PopupState::None {
        match state.ui_state.screen {
            Screen::Ingame => {
                // tile_info
                info::info_ui(state);

                if !state.input_handler.key_active(ActionType::ToggleGui) {
                    if let Some(map_info) = state.loop_store.map_info.as_ref().map(|v| v.0.clone())
                    {
                        let mut lock = map_info.blocking_lock();
                        let game_data = &mut lock.data;

                        let (selection_send, selection_recv) = oneshot::channel();

                        // tile_selections
                        tile_selection::tile_selections(state, game_data, selection_send);

                        if let Ok(id) = selection_recv.blocking_recv() {
                            state.ui_state.already_placed_at = None;

                            if state.ui_state.selected_tile_id == Some(id) {
                                state.ui_state.selected_tile_id = None;
                            } else {
                                state.ui_state.selected_tile_id = Some(id);
                            }
                        }

                        player::player(state, game_data);

                        // tile_config
                        tile_config::tile_config_ui(state, game_data);
                    }

                    let cursor_pos = math::screen_to_world(
                        window::window_size_double(&state.renderer.as_ref().unwrap().gpu.window),
                        state.input_handler.main_pos,
                        state.camera.get_pos(),
                    );

                    render_overlay_cached(
                        &state.resource_man,
                        state.renderer.as_mut().unwrap(),
                        state.ui_state.selected_tile_id,
                        DataMap::default(),
                        &mut state.ui_state.selected_tile_render_cache,
                        Matrix4::from_translation(vec3(
                            cursor_pos.x as Float,
                            cursor_pos.y as Float,
                            FAR,
                        )),
                        state.camera.get_matrix(),
                    );

                    if let Some((coord, ..)) = state.ui_state.linking_tile {
                        state.renderer.as_mut().unwrap().overlay_instances.push((
                            InstanceData::default().with_color_offset(colors::RED.to_linear()),
                            ModelId(state.resource_man.registry.model_ids.cube1x1),
                            GameMatrix::<true>::new(
                                make_line(
                                    HEX_GRID_LAYOUT.hex_to_world_pos(*coord),
                                    cursor_pos.truncate(),
                                    FAR,
                                ),
                                state.camera.get_matrix(),
                                Matrix4::IDENTITY,
                            ),
                            0,
                        ));
                    }
                }
            }
            Screen::MainMenu => *result = menu::main_menu(state, event_loop),
            Screen::MapLoad => {
                menu::map_menu(state);
            }
            Screen::Options => {
                menu::options_menu(state);
            }
            Screen::Paused => {
                menu::pause_menu(state);
            }
        }
    }

    match state.ui_state.popup.clone() {
        PopupState::None => {}
        PopupState::MapCreate => popup::map_create_popup(state),
        PopupState::MapDeleteConfirmation(map_name) => {
            popup::map_delete_popup(state, &map_name);
        }
        PopupState::InvalidName => {
            popup::invalid_name_popup(state);
        }
    }

    util::render_info_tip(state);

    state.renderer.as_mut().unwrap().tile_tints.insert(
        state.camera.pointing_at,
        colors::RED.with_alpha(0.2).to_linear(),
    );

    for coord in &state.ui_state.grouped_tiles {
        state
            .renderer
            .as_mut()
            .unwrap()
            .tile_tints
            .insert(*coord, colors::ORANGE.with_alpha(0.4).to_linear());
    }

    if let Some(start) = state.ui_state.paste_from {
        if start != state.camera.pointing_at {
            state.renderer.as_mut().unwrap().overlay_instances.push((
                InstanceData::default().with_color_offset(colors::LIGHT_BLUE.to_linear()),
                ModelId(state.resource_man.registry.model_ids.cube1x1),
                GameMatrix::<true>::new(
                    make_line(
                        HEX_GRID_LAYOUT.hex_to_world_pos(*start),
                        HEX_GRID_LAYOUT.hex_to_world_pos(*state.camera.pointing_at),
                        FAR,
                    ),
                    state.camera.get_matrix(),
                    Matrix4::IDENTITY,
                ),
                0,
            ));
        }

        let diff = state.camera.pointing_at - start;

        for (coord, id, data) in &state.ui_state.paste_content {
            let model_matrix = {
                let coord = *coord + diff;
                let p = HEX_GRID_LAYOUT.hex_to_world_pos(*coord);

                Matrix4::from_translation(vec3(p.x, p.y, FAR))
            };

            let cache = state
                .ui_state
                .paste_content_render_cache
                .entry(*coord)
                .or_default();
            render_overlay_cached(
                &state.resource_man,
                state.renderer.as_mut().unwrap(),
                Some(*id),
                data.clone().unwrap_or_default(),
                cache,
                model_matrix,
                state.camera.get_matrix(),
            );
        }
    }

    if state.input_handler.key_active(ActionType::Debug) {
        debug::debugger(state);
    }

    error::error_popup(state);
}
