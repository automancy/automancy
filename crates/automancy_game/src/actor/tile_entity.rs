use std::sync::Arc;

use automancy_data::{
    game::{coord::TileCoord, generic::DataMap, inventory::ItemStack},
    id::{Id, TileId},
    math::Int,
};
use hashbrown::HashSet;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use rand::RngCore;
use rhai::{Dynamic, Scope};
use thiserror::Error;

use crate::{
    actor::message::{GameMsg, TileMsg, TileResult, TileTransactionResult},
    resources::{ResourceManager, rhai_call_options, rhai_log_err, types::script::ScriptData},
    scripting::render::RenderCommand,
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
    data: DataMap,
    field_changes_since_render: HashSet<Id>,
}

impl TileActorState {
    fn new() -> Self {
        Self {
            data: DataMap::new(),
            field_changes_since_render: Default::default(),
        }
    }
}

impl TileActor {
    fn send_to_tile(&self, state: &mut TileActorState, coord: TileCoord, msg: TileMsg) {
        match self.game.send_message(GameMsg::SendTileMsg(coord, msg)) {
            Ok(_) => {}
            Err(_) => {
                state.field_changes_since_render.extend(state.data.keys().copied());
                state.data = Default::default();
            }
        }
    }

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
            }
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
            }
            TileTransactionResult::Consume {
                consumed,
                source_coord,
                root_coord,
            } => {
                self.send_to_tile(state, root_coord, TileMsg::TransactionResult { result: consumed });

                None
            }
        }
    }

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
            }
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
            }
        }
    }

    fn run_tile_script<Result: 'static, const SIZE: usize>(
        &self,
        state: &mut TileActorState,
        script: &ScriptData,
        args: [(&'static str, Dynamic); SIZE],
        function_name: &'static str,
    ) -> Option<Result> {
        fn random() -> Int {
            rand::rng().next_u32() as Int
        }

        let tile_def = self.resource_man.registry.tile_defs.get(&self.id)?;

        let input = rhai::Map::from_iter(
            [
                ("coord", Dynamic::from(self.coord)),
                ("id", Dynamic::from(self.id)),
                ("random", Dynamic::from_int(random())),
                ("setup", Dynamic::from(tile_def.data.clone())),
            ]
            .into_iter()
            .chain(args)
            .map(|(k, v)| (rhai::Identifier::from(k), v)),
        );

        let old_keys = state.data.keys().copied().collect::<HashSet<_>>();
        let mut rhai_state = Dynamic::from(std::mem::take(&mut state.data));
        let result = self.resource_man.engine.call_fn_with_options::<Dynamic>(
            rhai_call_options(&mut rhai_state),
            &mut Scope::new(),
            &script.ast,
            function_name,
            (input,),
        );
        state.data = rhai_state.cast::<DataMap>();
        for key in state.data.keys().copied() {
            if !old_keys.contains(&key) {
                state.field_changes_since_render.insert(key);
            }
        }

        match result {
            Ok(result) => result.try_cast::<Result>(),
            Err(err) => {
                rhai_log_err(function_name, &script.metadata.str_id, &err, Some(self.coord));
                None
            }
        }
    }

    fn transaction(
        &self,
        state: &mut TileActorState,
        stack: ItemStack,
        source_coord: TileCoord,
        source_id: TileId,
        root_coord: TileCoord,
        root_id: TileId,
    ) -> Option<GameMsg> {
        let tile = self.resource_man.registry.tile_defs.get(&self.id)?;

        if let Some(script) = tile.script.as_ref().and_then(|v| self.resource_man.scripts.get(v))
            && let Some(result) = self.run_tile_script(
                state,
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

    fn collect_render_commands(&self, state: &mut TileActorState, loading: bool, unloading: bool) -> Option<Vec<RenderCommand>> {
        let tile_def = self.resource_man.registry.tile_defs.get(&self.id)?;

        if let Some(script) = tile_def.script.as_ref().and_then(|v| self.resource_man.scripts.get(v)) {
            if !(loading || unloading) {
                return None;
            }

            let field_changes = std::mem::take(&mut state.field_changes_since_render);
            if let Some(result) = self.run_tile_script(
                state,
                script,
                [
                    ("loading", Dynamic::from_bool(loading)),
                    ("unloading", Dynamic::from_bool(unloading)),
                    ("field_changes", Dynamic::from(field_changes)),
                ],
                "tile_render",
            ) as Option<rhai::Array>
            {
                return Some(result.into_iter().flat_map(|v| v.try_cast::<RenderCommand>()).collect::<Vec<_>>());
            }
        }

        None
    }
}

#[derive(Error, Debug)]
pub enum TileActorError {
    #[error("the tile ID at {0} is no longer existent")]
    NonExistent(TileCoord),
}

impl Actor for TileActor {
    type Msg = TileMsg;
    type State = TileActorState;
    type Arguments = ();

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, _args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(TileActorState::new())
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match message {
            TileMsg::Tick { tick_count: _tick_count } => {
                let tile_def = self
                    .resource_man
                    .registry
                    .tile_defs
                    .get(&self.id)
                    .ok_or(Box::new(TileActorError::NonExistent(self.coord)))?;

                if let Some(script) = tile_def.script.as_ref().and_then(|v| self.resource_man.scripts.get(v))
                    && let Some(result) = self.run_tile_script(state, script, [], "handle_tick")
                {
                    self.handle_rhai_result(state, result);
                }
            }

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
            }
            TileMsg::TransactionResult { result } => {
                let tile_def = self
                    .resource_man
                    .registry
                    .tile_defs
                    .get(&self.id)
                    .ok_or(Box::new(TileActorError::NonExistent(self.coord)))?;

                if let Some(script) = tile_def.script.as_ref().and_then(|v| self.resource_man.scripts.get(v)) {
                    let _: Option<()> = self.run_tile_script(state, script, [("transferred", Dynamic::from(result))], "handle_transaction_result");
                }
            }
            TileMsg::ExtractRequest {
                requested_from_id,
                requested_from_coord,
            } => {
                let tile_def = self
                    .resource_man
                    .registry
                    .tile_defs
                    .get(&self.id)
                    .ok_or(Box::new(TileActorError::NonExistent(self.coord)))?;

                if let Some(script) = tile_def.script.as_ref().and_then(|v| self.resource_man.scripts.get(v))
                    && let Some(result) = self.run_tile_script(
                        state,
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
            }

            TileMsg::CollectRenderCommands { reply, loading, unloading } => {
                reply.send(self.collect_render_commands(state, loading, unloading))?;
            }
            TileMsg::GetTileConfigUi(reply) => {
                let tile_def = self
                    .resource_man
                    .registry
                    .tile_defs
                    .get(&self.id)
                    .ok_or(Box::new(TileActorError::NonExistent(self.coord)))?;

                if let Some(script) = tile_def.script.as_ref().and_then(|v| self.resource_man.scripts.get(v)) {
                    if let Some(result) = self.run_tile_script(state, script, [], "tile_config") {
                        reply.send(Some(result))?;
                    } else {
                        reply.send(None)?;
                    }
                }
            }

            TileMsg::GetData(reply) => {
                reply.send(state.data.clone())?;
            }
            TileMsg::GetDatum(key, reply) => {
                reply.send(state.data.get(key).cloned())?;
            }
            TileMsg::SetData(data) => {
                state.field_changes_since_render.extend(state.data.keys().copied());
                state.data = data;
                state.field_changes_since_render.extend(state.data.keys().copied());
            }
            TileMsg::SetDatum(key, value) => {
                state.field_changes_since_render.insert(key);
                state.data.set(key, value);
            }
            TileMsg::TakeData(reply) => {
                state.field_changes_since_render.extend(state.data.keys().copied());
                reply.send(std::mem::take(&mut state.data))?;
            }
            TileMsg::RemoveDatum(key) => {
                state.field_changes_since_render.insert(key);
                state.data.remove(key);
            }
            TileMsg::ReadData(f) => {
                state.field_changes_since_render.extend(state.data.keys().copied());
                f(&mut state.data);
            }
        }

        Ok(())
    }
}
