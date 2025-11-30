use std::{collections::BTreeMap, time::Instant};

use automancy_data::{
    game::coord::TileCoord,
    rendering::{colors, colors::PackedRgba, draw::GameUniformData},
};
use automancy_game::state::AutomancyGameState;

use crate::{GameInstanceId, gpu, renderer, renderer::WorldGameInstanceIndex};

pub struct GameRenderer {
    pub tile_tints: BTreeMap<TileCoord, PackedRgba>,
    last_tile_tints: BTreeMap<TileCoord, PackedRgba>,
}

impl Default for GameRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl GameRenderer {
    pub fn new() -> Self {
        Self {
            tile_tints: Default::default(),
            last_tile_tints: Default::default(),
        }
    }

    #[inline]
    pub fn update_tile_hints(&mut self, render_state: &mut renderer::AutomancyRenderState) {
        let last_tile_tints = std::mem::take(&mut self.last_tile_tints);
        let tile_tints = std::mem::take(&mut self.tile_tints);

        for &coord in last_tile_tints.keys() {
            if tile_tints.contains_key(&coord) {
                continue;
            };

            let Some(ids) = render_state.game_map_draw_ids.get(&(coord, ())) else {
                continue;
            };

            for &(model_id, render_id) in ids {
                render_state.game_map_instance_man.modify_instances(
                    &render_state.model_man,
                    GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    |_, instance| {
                        instance.color_offset = colors::TRANSPARENT.packed;
                    },
                );
            }
        }

        for (&coord, &tint) in &tile_tints {
            let Some(ids) = render_state.game_map_draw_ids.get(&(coord, ())) else {
                continue;
            };

            for &(model_id, render_id) in ids {
                render_state.game_map_instance_man.modify_instances(
                    &render_state.model_man,
                    GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    |_, instance| {
                        instance.color_offset = tint;
                    },
                );
            }
        }

        self.last_tile_tints = tile_tints;
    }

    #[inline]
    #[track_caller]
    pub fn render(
        &mut self,
        res: &mut gpu::RenderResources,
        render_state: &mut renderer::AutomancyRenderState,
        game_state: &AutomancyGameState,
        frame_start: Instant,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let game_uniform = GameUniformData {
            camera_pos: game_state.camera.view_center,
            camera_bounds: game_state.camera.bounding_rect,
            view_matrix: game_state.camera.transform,
            ..Default::default()
        };

        {
            #[cfg(feature = "profile")]
            profiling::scope!("draw_game_map");

            render_state.game_map_instance_man.pre_render(
                &res.device,
                &res.queue,
                &res.global_res,
                &mut res.main_game_res,
                &mut render_state.model_man,
                frame_start,
            );

            // TODO make animations global
            render_state
                .game_map_instance_man
                .animations
                .upload(&res.queue, &res.main_game_res.game_pipeline.animation_matrix_buffer);

            let draw_calls = render_state
                .game_map_instance_man
                .collect_draw_calls(&res.device, &res.queue, &mut res.main_game_res);

            res.queue.submit([]);

            res.main_game_res.render(
                &res.queue,
                &res.global_res,
                encoder,
                [draw_calls.opaque.buffer.len() as u32, draw_calls.non_opaque.buffer.len() as u32],
                (
                    gpu::data::GpuGameUniformData::new(&game_uniform),
                    gpu::data::GpuGameLightingUniformData::new(&game_uniform),
                    gpu::data::GpuPostProcessingUniformData {
                        ..Default::default()
                    },
                ),
            );
        }

        {
            #[cfg(feature = "profile")]
            profiling::scope!("draw_overlay");

            render_state.overlay_instance_man.pre_render(
                &res.device,
                &res.queue,
                &res.global_res,
                &mut res.overlay_game_res,
                &mut render_state.model_man,
                frame_start,
            );

            render_state
                .overlay_instance_man
                .animations
                .upload(&res.queue, &res.overlay_game_res.game_pipeline.animation_matrix_buffer);

            let draw_calls = render_state
                .overlay_instance_man
                .collect_draw_calls(&res.device, &res.queue, &mut res.overlay_game_res);

            res.queue.submit([]);

            res.overlay_game_res.render(
                &res.queue,
                &res.global_res,
                encoder,
                [draw_calls.opaque.buffer.len() as u32, draw_calls.non_opaque.buffer.len() as u32],
                (
                    gpu::data::GpuGameUniformData::new(&game_uniform),
                    gpu::data::GpuGameLightingUniformData::new(&game_uniform),
                    gpu::data::GpuPostProcessingUniformData {
                        ..Default::default()
                    },
                ),
            );
        }
    }
}
