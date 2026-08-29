use std::collections::{BTreeMap, BTreeSet};

use automancy_data::{
    game::{
        coord::{TileCoord, TileCoordBounds},
        generic::DataMap,
    },
    id::{ModelId, RenderId, UiRenderId},
    math::{Matrix4, Vec2},
    rendering::{GenericModel, draw::GameDrawInstance},
};
use automancy_game::{
    actor::{
        game_entity,
        message::{GameMsg, GameRenderCommands},
        tile_entity,
        tile_entity::TileActorState,
    },
    scripting_rhai::render,
    state::AutomancyGameState,
};
use ractor::rpc::CallResult;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};

use crate::{GameInstanceId, GameInstanceManager, ModelManager, gpu};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldGameInstanceIndex {
    pub coord: TileCoord,
    pub render_id: RenderId,
}
pub type InstanceIdMap<Index> = BTreeMap<(TileCoord, Index), BTreeSet<(ModelId, RenderId)>>;

pub mod util {
    use automancy_data::{game::coord::TileCoord, rendering::draw::GameDrawInstance};
    use automancy_game::script::RenderCommand;

    use crate::{
        GameInstanceManager, ModelManager,
        renderer::{InstanceIdMap, WorldGameInstanceIndex},
    };

    #[cfg_attr(feature = "profile", profiling::function)]
    #[inline]
    pub fn handle_render_command<Index: crate::InstanceIndex>(
        instance_man: &mut GameInstanceManager<WorldGameInstanceIndex>,
        command: RenderCommand,
        coord: TileCoord,
        model_man: &ModelManager,
        draw_ids: &mut InstanceIdMap<Index>,
        index: Index,
        instance: GameDrawInstance,
    ) {
        match command {
            RenderCommand::Track {
                render_id,
                model_id,
            } => {
                draw_ids.entry((coord, index)).or_default().insert((model_id, render_id));

                instance_man.insert(
                    model_man,
                    crate::GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    instance,
                );
            },
            RenderCommand::Transform {
                render_id,
                model_id,
                model_matrix,
            } => {
                instance_man.set_matrix(
                    model_man,
                    crate::GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    (Some(model_matrix), None),
                );
            },
            RenderCommand::Untrack {
                render_id,
                model_id,
            } => {
                draw_ids.entry((coord, index)).or_default().remove(&(model_id, render_id));

                instance_man.remove(crate::GameInstanceId {
                    model_id,
                    index: WorldGameInstanceIndex {
                        coord,
                        render_id,
                    },
                });
            },
        }
    }
}

#[derive(Debug)]
pub struct AutomancyRenderState {
    pub model_man: ModelManager,
    pub game_map_instance_man: GameInstanceManager<WorldGameInstanceIndex>,
    pub game_map_draw_ids: InstanceIdMap<()>,

    game_render_commands_tx: mpsc::UnboundedSender<[GameRenderCommands; 2]>,
    game_render_commands_rx: mpsc::UnboundedReceiver<[GameRenderCommands; 2]>,

    pub(crate) game_culling_bounds_tx: watch::Sender<TileCoordBounds>,
    pub(crate) game_culling_bounds_rx: watch::Receiver<TileCoordBounds>,

    game_render_commands_update_handle: Option<JoinHandle<()>>,

    pub(crate) overlay_instance_man: GameInstanceManager<WorldGameInstanceIndex>,
    overlay_draw_ids: InstanceIdMap<UiRenderId>,
    overlay_instance_map: BTreeMap<UiRenderId, BTreeSet<(TileCoord, GenericModel)>>,
    overlay_floor_added: bool,
}

impl Default for AutomancyRenderState {
    fn default() -> Self {
        let (game_render_commands_tx, game_render_commands_rx) = mpsc::unbounded_channel();
        let (game_culling_bounds_tx, game_culling_bounds_rx) = watch::channel(TileCoordBounds::Empty);

        Self {
            model_man: Default::default(),
            game_map_instance_man: GameInstanceManager::new("GAME_WORLD"),
            game_map_draw_ids: Default::default(),

            game_render_commands_tx,
            game_render_commands_rx,

            game_culling_bounds_tx,
            game_culling_bounds_rx,

            game_render_commands_update_handle: None,

            overlay_instance_man: GameInstanceManager::new("GAME_OVERLAY"),
            overlay_draw_ids: Default::default(),
            overlay_instance_map: Default::default(),
            overlay_floor_added: false,
        }
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl AutomancyRenderState {
    fn update_render_commands(&mut self) {
        while let Ok(batches) = self.game_render_commands_rx.try_recv() {
            for batch in batches {
                for (coord, commands) in batch {
                    for command in commands {
                        crate::renderer::util::handle_render_command(
                            &mut self.game_map_instance_man,
                            command,
                            coord,
                            &self.model_man,
                            &mut self.game_map_draw_ids,
                            (),
                            GameDrawInstance::default(),
                        );
                    }
                }

                self.game_map_instance_man.flush_removal(&self.model_man);
            }
        }
    }

    fn update_overlay_z_floor(&mut self, game_state: &AutomancyGameState) {
        // add a model at z=0
        let ui_render_id = game_state.resource_man.registry.render_ids.overlay_z_zero_floor;

        if !self.overlay_floor_added {
            let model = GenericModel::Plain(game_state.resource_man.registry.model_ids.cube1x1);
            self.add_overlay_model(
                game_state,
                TileCoord::ZERO,
                model,
                ui_render_id,
                &mut DataMap::new(),
                GameDrawInstance {
                    alpha: 0.0,
                    ..Default::default()
                },
            );
            self.overlay_floor_added = true;
        }

        let view_size = Vec2::from(game_state.camera.bounding_rect.size());

        // fill overlay with a full-screen rect
        self.set_overlay_matrix(
            TileCoord::ZERO,
            ui_render_id,
            (
                Some(Matrix4::translation_3d(game_state.camera.pos.with_z(-1.0001)) * Matrix4::scaling_3d((view_size * 4.0).with_z(1.0))),
                None,
            ),
        );
    }

    pub fn update(&mut self, game_state: &AutomancyGameState) {
        {
            #[cfg(feature = "profile")]
            profiling::function_scope!("update_culling_bounds");

            let culling_bounds = game_state.camera.culling_bounds;
            self.game_culling_bounds_tx.send_if_modified(|curr| {
                if *curr != culling_bounds {
                    *curr = culling_bounds;
                    return true;
                }

                false
            });
        }

        if self.game_render_commands_update_handle.is_none() {
            let game_handle = game_state.game_handle.clone();

            let game_render_commands_tx = self.game_render_commands_tx.clone();
            let mut game_culling_bounds_rx = self.game_culling_bounds_rx.clone();

            self.game_render_commands_update_handle = Some(game_state.tokio.spawn(async move {
                let mut interval = tokio::time::interval(game_entity::TICK_INTERVAL);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                let mut culling_bounds = TileCoordBounds::Empty;

                loop {
                    tokio::select! {
                        Ok(()) = game_culling_bounds_rx.changed() => {
                            culling_bounds = *game_culling_bounds_rx.borrow();
                        }
                        // if culling bounds didn't change, throttle the rate to once a tick
                        _ = interval.tick() => {}
                    }

                    if culling_bounds != TileCoordBounds::Empty {
                        match game_handle
                            .call(
                                |reply| GameMsg::CollectRenderCommands {
                                    culling_bounds,
                                    reply,
                                },
                                None,
                            )
                            .await
                        {
                            Ok(CallResult::Success(render_commands)) => match game_render_commands_tx.send(render_commands) {
                                Ok(_) => {},
                                Err(err) => {
                                    log::error!("Error when collecting RenderCommand for rendering: {err}");
                                },
                            },
                            _ => {
                                return;
                            },
                        }
                    }
                }
            }));
        }

        self.update_overlay_z_floor(game_state);
        self.update_render_commands();
    }

    #[inline]
    pub fn add_overlay_model(
        &mut self,
        game_state: &AutomancyGameState,
        coord: TileCoord,
        model: GenericModel,
        ui_render_id: UiRenderId,
        data_map: &mut DataMap,
        instance: GameDrawInstance,
    ) {
        match model {
            GenericModel::None => {},
            GenericModel::Plain(model_id) => {
                let render_id = game_state.resource_man.registry.render_ids.overlay;

                self.overlay_instance_man.insert(
                    &self.model_man,
                    GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    instance,
                );
                self.overlay_draw_ids
                    .entry((coord, ui_render_id))
                    .or_default()
                    .insert((model_id, render_id));
                self.overlay_instance_map.entry(ui_render_id).or_default().insert((coord, model));
            },
            GenericModel::Item(item_id) => {
                let model_id = game_state.resource_man.item_model_or_missing(item_id);
                let render_id = game_state.resource_man.registry.render_ids.overlay;

                self.overlay_instance_man.insert(
                    &self.model_man,
                    GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    instance,
                );
                self.overlay_draw_ids
                    .entry((coord, ui_render_id))
                    .or_default()
                    .insert((model_id, render_id));
                self.overlay_instance_map.entry(ui_render_id).or_default().insert((coord, model));
            },
            GenericModel::Tile(tile_id) => {
                let commands = if !tile_id.is_none() {
                    if let Some(tile_def) = game_state.resource_man.registry.tile_defs.get(&tile_id)
                        && let Some(script) = game_state.resource_man.rhai_scripts.get(&tile_def.script)
                    {
                        let mut state = TileActorState {
                            data: std::mem::take(data_map),
                            rhai: Some(script.clone()),
                            rendering: true,
                            ..Default::default()
                        };

                        if let Some(commands) =
                            tile_entity::collect_render_commands(&game_state.resource_man, tile_id, coord, &mut state, true, false)
                        {
                            *data_map = state.data;

                            commands
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                } else {
                    render::util::track_none(&game_state.resource_man, coord).to_vec()
                };

                for command in commands {
                    crate::renderer::util::handle_render_command(
                        &mut self.overlay_instance_man,
                        command,
                        coord,
                        &self.model_man,
                        &mut self.overlay_draw_ids,
                        ui_render_id,
                        instance,
                    );
                }

                self.overlay_instance_map.entry(ui_render_id).or_default().insert((coord, model));
            },
        }
    }

    #[inline]
    pub fn remove_overlay_model(&mut self, ui_render_id: UiRenderId) {
        if let Some(list) = self.overlay_instance_map.remove(&ui_render_id) {
            for &(coord, _) in &list {
                if let Some(ids) = self.overlay_draw_ids.remove(&(coord, ui_render_id)) {
                    for (model_id, render_id) in ids {
                        self.overlay_instance_man.remove(GameInstanceId {
                            model_id,
                            index: WorldGameInstanceIndex {
                                coord,
                                render_id,
                            },
                        });
                    }
                }
            }
        }
    }

    #[inline]
    pub fn flush_overlay_model(&mut self) {
        self.overlay_instance_man.flush_removal(&self.model_man);
    }

    #[inline]
    pub fn modify_overlay_instances(
        &mut self,
        coord: TileCoord,
        ui_render_id: UiRenderId,
        mut f: impl FnMut(GameInstanceId<WorldGameInstanceIndex>, &mut gpu::data::GpuGameDrawInstance),
    ) {
        if let Some(ids) = self.overlay_draw_ids.get(&(coord, ui_render_id)) {
            for &(model_id, render_id) in ids {
                self.overlay_instance_man.modify_instances(
                    &self.model_man,
                    GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    &mut f,
                );
            }
        }
    }

    #[inline]
    pub fn set_overlay_matrix(&mut self, coord: TileCoord, ui_render_id: UiRenderId, matrix: (Option<Matrix4>, Option<Matrix4>)) {
        if let Some(ids) = self.overlay_draw_ids.get(&(coord, ui_render_id)) {
            for &(model_id, render_id) in ids {
                self.overlay_instance_man.set_matrix(
                    &self.model_man,
                    GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    matrix,
                );
            }
        }
    }

    #[inline]
    pub fn mul_overlay_matrix_left(&mut self, coord: TileCoord, ui_render_id: UiRenderId, matrix: (Option<Matrix4>, Option<Matrix4>)) {
        if let Some(ids) = self.overlay_draw_ids.get(&(coord, ui_render_id)) {
            for &(model_id, render_id) in ids {
                self.overlay_instance_man.mul_matrix_left(
                    &self.model_man,
                    GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    matrix,
                );
            }
        }
    }

    #[inline]
    pub fn mul_overlay_matrix_right(&mut self, coord: TileCoord, ui_render_id: UiRenderId, matrix: (Option<Matrix4>, Option<Matrix4>)) {
        if let Some(ids) = self.overlay_draw_ids.get(&(coord, ui_render_id)) {
            for &(model_id, render_id) in ids {
                self.overlay_instance_man.mul_matrix_right(
                    &self.model_man,
                    GameInstanceId {
                        model_id,
                        index: WorldGameInstanceIndex {
                            coord,
                            render_id,
                        },
                    },
                    matrix,
                );
            }
        }
    }
}
