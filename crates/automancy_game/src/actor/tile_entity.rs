use std::sync::Arc;

use automancy_data::{
    game::{coord::TileCoord, generic::DataMap, inventory::ItemStack},
    id::{Id, TileId},
    id_map::IdSet,
    math::Int,
};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use rand::Rng;
use rhai::{Dynamic, Scope};
use thiserror::Error;

use crate::{
    actor::message::{GameMsg, TileMsg, TileResult, TileTransactionResult},
    resources::{ResourceManager, types::script::RhaiScriptData},
    script::RenderCommand,
    scripting_rhai,
};

#[derive(Debug, Clone)]
pub struct TileActor {
    /// a handle to the game.
    pub game: ActorRef<GameMsg>,
    pub resource_man: Arc<ResourceManager>,

    pub id: TileId,
    pub coord: TileCoord,
}

#[derive(Debug, Clone)]
pub struct TileActorState {
    pub data: DataMap,
    pub rhai: Option<RhaiScriptData>,
    pub rendering: bool,
    pub field_changes_since_render: IdSet<Id>,
}

impl Default for TileActorState {
    fn default() -> Self {
        Self {
            data: DataMap::new(),
            rhai: None,
            rendering: false,
            field_changes_since_render: Default::default(),
        }
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl TileActor {
    #[inline]
    fn send_to_tile(&self, state: &mut TileActorState, coord: TileCoord, msg: TileMsg) {
        match self.game.send_message(GameMsg::SendTileMsg(coord, msg)) {
            Ok(_) => {},
            Err(_) => {
                state.field_changes_since_render.extend(state.data.keys());
                state.data = Default::default();
            },
        }
    }

    #[inline]
    fn handle_rhai_transaction_result(&self, state: &mut TileActorState, result: TileTransactionResult) -> Option<GameMsg> {
        match result {
            TileTransactionResult::PassOn {
                coord,
                stack,
                source_coord,
                root_coord,
                root_id,
            } => {
                self.send_to_tile(
                    state,
                    coord,
                    TileMsg::Transaction {
                        stack,
                        source_id: self.id,
                        source_coord: self.coord,
                        root_id,
                        root_coord,
                        hidden: false,
                    },
                );

                None
            },
            TileTransactionResult::Proxy {
                coord,
                stack,
                source_coord,
                source_id,
                root_coord,
                root_id,
            } => {
                self.send_to_tile(
                    state,
                    coord,
                    TileMsg::Transaction {
                        stack,
                        source_id,
                        source_coord,
                        root_id,
                        root_coord,
                        hidden: false,
                    },
                );

                None
            },
            TileTransactionResult::Consume {
                consumed,
                source_coord,
                root_coord,
            } => {
                self.send_to_tile(
                    state,
                    root_coord,
                    TileMsg::TransactionResult {
                        result: consumed,
                    },
                );

                None
            },
        }
    }

    #[inline]
    fn handle_rhai_result(&self, state: &mut TileActorState, result: TileResult) {
        match result {
            TileResult::MakeTransaction {
                coord,
                source_id,
                source_coord,
                stacks,
            } => {
                for stack in stacks {
                    self.send_to_tile(
                        state,
                        coord,
                        TileMsg::Transaction {
                            stack,
                            source_coord,
                            source_id,
                            root_coord: source_coord,
                            root_id: source_id,
                            hidden: false,
                        },
                    );
                }
            },
            TileResult::MakeExtractRequest {
                coord,
                requested_from_id,
                requested_from_coord,
                on_fail_action,
            } => {
                self.send_to_tile(
                    state,
                    coord,
                    TileMsg::ExtractRequest {
                        requested_from_id,
                        requested_from_coord,
                    },
                );
            },
        }
    }

    #[inline]
    fn transaction(
        &self,
        state: &mut TileActorState,
        stack: ItemStack,
        source_coord: TileCoord,
        source_id: TileId,
        root_coord: TileCoord,
        root_id: TileId,
    ) -> Option<GameMsg> {
        if let Some(script) = &state.rhai
            && let Some(result) = run_tile_script(
                &self.resource_man,
                self.id,
                self.coord,
                &mut state.data,
                &mut state.field_changes_since_render,
                script,
                [
                    ("source_coord", Dynamic::from(source_coord)),
                    ("source_id", Dynamic::from(source_id)),
                    ("root_coord", Dynamic::from(root_coord)),
                    ("root_id", Dynamic::from(root_id)),
                    ("stack", Dynamic::from(stack)),
                ],
                "handle_transaction",
            )
        {
            return self.handle_rhai_transaction_result(state, result);
        }

        None
    }

    #[inline]
    pub fn collect_render_commands(&self, state: &mut TileActorState, loading: bool, unloading: bool) -> Option<Vec<RenderCommand>> {
        collect_render_commands(&self.resource_man, self.id, self.coord, state, loading, unloading)
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
#[allow(clippy::too_many_arguments)]
#[inline]
pub fn run_tile_script<Result: 'static, const SIZE: usize>(
    resource_man: &ResourceManager,
    id: TileId,
    coord: TileCoord,
    data: &mut DataMap,
    field_changes_since_render: &mut IdSet<Id>,
    script: &RhaiScriptData,
    args: [(&'static str, Dynamic); SIZE],
    function_name: &'static str,
) -> Option<Result> {
    fn random() -> Int {
        rand::rng().next_u32() as Int
    }

    let tile_def = resource_man.registry.tile_defs.get(&id)?;

    let input = rhai::Map::from_iter(
        [
            ("coord", Dynamic::from(coord)),
            ("id", Dynamic::from(id)),
            ("random", Dynamic::from_int(random())),
            ("setup", Dynamic::from(tile_def.data.clone())),
        ]
        .into_iter()
        .chain(args)
        .map(|(k, v)| (rhai::Identifier::from(k), v)),
    );

    let old_keys = data.keys().collect::<IdSet<_>>();
    let mut rhai_state = Dynamic::from(std::mem::take(data));
    let result = resource_man.rhai.call_fn_with_options::<Dynamic>(
        scripting_rhai::rhai_call_options(&mut rhai_state),
        &mut Scope::new(),
        &script.ast,
        function_name,
        (input,),
    );
    *data = rhai_state.cast::<DataMap>();
    for key in data.keys() {
        if !old_keys.contains(&key) {
            field_changes_since_render.insert(key);
        }
    }

    match result {
        Ok(result) => result.try_cast::<Result>(),
        Err(err) => {
            scripting_rhai::rhai_log_err(function_name, &script.metadata.str_id, &err, Some(coord));
            None
        },
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
#[allow(clippy::too_many_arguments)]
#[inline]
pub fn collect_render_commands(
    resource_man: &ResourceManager,
    id: TileId,
    coord: TileCoord,
    state: &mut TileActorState,
    loading: bool,
    unloading: bool,
) -> Option<Vec<RenderCommand>> {
    if !(loading || unloading || !state.field_changes_since_render.is_empty()) {
        return None;
    }

    if let Some(script) = &state.rhai {
        let field_changes = std::mem::take(&mut state.field_changes_since_render);
        if let Some(result) = run_tile_script(
            resource_man,
            id,
            coord,
            &mut state.data,
            &mut state.field_changes_since_render,
            script,
            [
                ("loading", Dynamic::from_bool(loading)),
                ("unloading", Dynamic::from_bool(unloading)),
                ("field_changes", Dynamic::from(field_changes)),
            ],
            "tile_render",
        ) as Option<rhai::Array>
        {
            return Some(
                result
                    .into_iter()
                    .flat_map(|v| v.try_cast::<RenderCommand>())
                    .map(|command| match command {
                        RenderCommand::Track {
                            render_id,
                            model_id,
                        } => {
                            let model_id = resource_man.model_or_missing_tile(model_id);
                            RenderCommand::Track {
                                render_id,
                                model_id,
                            }
                        },
                        RenderCommand::Transform {
                            render_id,
                            model_id,
                            model_matrix,
                        } => {
                            let model_id = resource_man.model_or_missing_tile(model_id);
                            RenderCommand::Transform {
                                render_id,
                                model_id,
                                model_matrix,
                            }
                        },
                        RenderCommand::Untrack {
                            render_id,
                            model_id,
                        } => {
                            let model_id = resource_man.model_or_missing_tile(model_id);
                            RenderCommand::Untrack {
                                render_id,
                                model_id,
                            }
                        },
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }

    None
}

#[derive(Debug, Error)]
pub enum TileActorError {
    #[error("the tile Id at {0} is no longer existent")]
    NonExistent(TileCoord),
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl Actor for TileActor {
    type Msg = TileMsg;
    type State = TileActorState;
    type Arguments = ();

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, _args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        let tile_def = self
            .resource_man
            .registry
            .tile_defs
            .get(&self.id)
            .ok_or(Box::new(TileActorError::NonExistent(self.coord)))?;

        let mut state = TileActorState::default();

        if let Some(data) = self.resource_man.rhai_scripts.get(&tile_def.script) {
            state.rhai = Some(data.clone());
        }

        Ok(state)
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match message {
            TileMsg::StartRendering(reply) => {
                state.rendering = true;
                let commands = self.collect_render_commands(state, true, false).unwrap_or_default();
                reply.send(commands)?;
            },
            TileMsg::StopRendering(reply) => {
                state.rendering = false;
                let commands = self.collect_render_commands(state, false, true).unwrap_or_default();
                reply.send(commands)?;
            },

            TileMsg::Tick {
                tick_count: _tick_count,
            } => {
                if let Some(script) = &state.rhai
                    && let Some(result) = run_tile_script(
                        &self.resource_man,
                        self.id,
                        self.coord,
                        &mut state.data,
                        &mut state.field_changes_since_render,
                        script,
                        [],
                        "handle_tick",
                    )
                {
                    self.handle_rhai_result(state, result);
                }

                if state.rendering {
                    let commands = self.collect_render_commands(state, false, false).unwrap_or_default();

                    if !commands.is_empty() {
                        self.game
                            .cast(GameMsg::PushRenderCommands {
                                coord: self.coord,
                                commands,
                            })
                            .unwrap();
                    }
                }
            },

            TileMsg::Transaction {
                stack,
                source_coord,
                source_id,
                root_coord,
                root_id,
                hidden,
            } => {
                if let Some(record) = self.transaction(state, stack, source_coord, source_id, root_coord, root_id)
                    && !hidden
                {
                    self.game.send_message(record)?;
                }
            },
            TileMsg::TransactionResult {
                result,
            } => {
                if let Some(script) = &state.rhai {
                    let _: Option<()> = run_tile_script(
                        &self.resource_man,
                        self.id,
                        self.coord,
                        &mut state.data,
                        &mut state.field_changes_since_render,
                        script,
                        [("transferred", Dynamic::from(result))],
                        "handle_transaction_result",
                    );
                }
            },
            TileMsg::ExtractRequest {
                requested_from_id,
                requested_from_coord,
            } => {
                if let Some(script) = &state.rhai
                    && let Some(result) = run_tile_script(
                        &self.resource_man,
                        self.id,
                        self.coord,
                        &mut state.data,
                        &mut state.field_changes_since_render,
                        script,
                        [
                            ("requested_from_coord", Dynamic::from(requested_from_coord)),
                            ("requested_from_id", Dynamic::from(requested_from_id)),
                        ],
                        "handle_extract_request",
                    )
                {
                    self.handle_rhai_result(state, result);
                }
            },

            TileMsg::GetTileConfigUi(reply) => {
                if let Some(script) = &state.rhai {
                    if let Some(result) = run_tile_script(
                        &self.resource_man,
                        self.id,
                        self.coord,
                        &mut state.data,
                        &mut state.field_changes_since_render,
                        script,
                        [],
                        "tile_config",
                    ) {
                        reply.send(Some(result))?;
                    } else {
                        reply.send(None)?;
                    }
                }
            },

            TileMsg::GetData(reply) => {
                reply.send(state.data.clone())?;
            },
            TileMsg::SetData(data) => {
                state.field_changes_since_render.extend(state.data.keys());
                state.data = *data;
                state.field_changes_since_render.extend(state.data.keys());
            },
            TileMsg::TakeData(reply) => {
                state.field_changes_since_render.extend(state.data.keys());
                reply.send(std::mem::take(&mut state.data))?;
            },
            TileMsg::SetDatum(id, datum) => {
                state.data.set(id, datum);
            },
            TileMsg::RemoveDatum(id, datum) => {
                state.data.remove(id, datum);
            },
            TileMsg::ChangeData(change) => {
                state.data.handle_change(change);
            },
            TileMsg::FnData(f) => {
                state.field_changes_since_render.extend(state.data.keys());
                f(&mut state.data);
            },
            TileMsg::FnDataStatic(f) => {
                state.field_changes_since_render.extend(state.data.keys());
                f(&mut state.data);
            },
        }

        Ok(())
    }
}
