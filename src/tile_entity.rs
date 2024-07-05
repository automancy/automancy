use std::mem;
use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use rand::{thread_rng, RngCore};
use rhai::{Dynamic, Scope};

use automancy_defs::id::Id;
use automancy_defs::{coord::TileCoord, stack::ItemStack};
use automancy_resources::data::{Data, DataMap};
use automancy_resources::rhai_ui::RhaiUiUnit;
use automancy_resources::types::function::{
    OnFailAction, RhaiDataMap, TileResult, TileTransactionResult,
};
use automancy_resources::{rhai_call_options, rhai_log_err, ResourceManager};
use thiserror::Error;

use crate::game::{GameSystemMessage, TickUnit};
use crate::tile_entity::TileEntityMsg::*;

#[derive(Debug, Clone)]
pub struct TileEntity {
    /// The ID of the tile entity.
    pub id: Id,
    /// The coordinates of the tile entity.
    pub coord: TileCoord,
    /// The handle to the Resource Manager
    pub resource_man: Arc<ResourceManager>,
}

/// Represents a tile entity's state. A tile entity is the actor that allows the tile to take, process, and output resources.
#[derive(Debug, Clone)]
pub struct TileEntityState {
    /// A handle to the game.
    game: ActorRef<GameSystemMessage>,

    /// The data map stored by the tile.
    data: RhaiDataMap,

    /// Rhai scope
    scope: Option<Scope<'static>>,
}

impl TileEntityState {
    fn new(game: ActorRef<GameSystemMessage>) -> Self {
        Self {
            game,

            data: Default::default(),

            scope: Default::default(),
        }
    }
}

#[derive(Debug)]
pub enum TileEntityMsg {
    Tick {
        tick_count: TickUnit,
    },
    Transaction {
        stack: ItemStack,
        source_coord: TileCoord,
        source_id: Id,
        root_coord: TileCoord,
        root_id: Id,
        hidden: bool,
    },
    TransactionResult {
        result: ItemStack,
    },
    ExtractRequest {
        requested_from_id: Id,
        requested_from_coord: TileCoord,
    },
    SetData(DataMap),
    SetDataValue(Id, Data),
    RemoveData(Id),
    TakeData(RpcReplyPort<DataMap>),
    GetData(RpcReplyPort<DataMap>),
    GetDataValue(Id, RpcReplyPort<Option<Data>>),
    GetDataWithCoord(RpcReplyPort<(TileCoord, DataMap)>),
    GetTileConfigUi(RpcReplyPort<Option<RhaiUiUnit>>),
}

impl TileEntity {
    fn handle_rhai_transaction_result(
        &self,
        state: &mut TileEntityState,
        result: TileTransactionResult,
    ) -> Option<GameSystemMessage> {
        match result {
            TileTransactionResult::PassOn {
                coord,
                stack,
                source_coord,
                root_coord,
                root_id,
            } => {
                send_to_tile(
                    state,
                    self.coord,
                    coord,
                    Transaction {
                        stack,
                        source_id: self.id,
                        source_coord: self.coord,
                        root_id,
                        root_coord,
                        hidden: false,
                    },
                    OnFailAction::None,
                );

                Some(GameSystemMessage::RecordTransaction(
                    stack,
                    source_coord,
                    self.coord,
                ))
            }
            TileTransactionResult::Proxy {
                coord,
                stack,
                source_coord,
                source_id,
                root_coord,
                root_id,
            } => {
                send_to_tile(
                    state,
                    self.coord,
                    coord,
                    Transaction {
                        stack,
                        source_id,
                        source_coord,
                        root_id,
                        root_coord,
                        hidden: false,
                    },
                    OnFailAction::None,
                );

                Some(GameSystemMessage::RecordTransaction(
                    stack, self.coord, coord,
                ))
            }
            TileTransactionResult::Consume {
                consumed,
                source_coord,
                root_coord,
            } => {
                send_to_tile(
                    state,
                    self.coord,
                    root_coord,
                    TransactionResult { result: consumed },
                    OnFailAction::None,
                );

                Some(GameSystemMessage::RecordTransaction(
                    consumed,
                    source_coord,
                    self.coord,
                ))
            }
        }
    }

    fn handle_rhai_result(&self, state: &mut TileEntityState, result: TileResult) {
        match result {
            TileResult::MakeTransaction {
                coord,
                source_id,
                source_coord,
                stacks,
            } => {
                for stack in stacks {
                    send_to_tile(
                        state,
                        source_coord,
                        coord,
                        Transaction {
                            stack,
                            source_coord,
                            source_id,
                            root_coord: source_coord,
                            root_id: source_id,
                            hidden: false,
                        },
                        OnFailAction::None,
                    );
                }
            }
            TileResult::MakeExtractRequest {
                coord,
                requested_from_id,
                requested_from_coord,
                on_fail_action,
            } => {
                send_to_tile(
                    state,
                    requested_from_coord,
                    coord,
                    ExtractRequest {
                        requested_from_id,
                        requested_from_coord,
                    },
                    on_fail_action,
                );
            }
        }
    }

    fn transaction(
        &self,
        state: &mut TileEntityState,
        stack: ItemStack,
        source_coord: TileCoord,
        source_id: Id,
        root_coord: TileCoord,
        root_id: Id,
    ) -> Option<GameSystemMessage> {
        let tile = self.resource_man.registry.tiles.get(&self.id)?;

        if let Some((ast, default_scope, function_id)) = tile
            .function
            .as_ref()
            .and_then(|v| self.resource_man.functions.get(v))
        {
            let scope = state
                .scope
                .get_or_insert_with(|| default_scope.clone_visible());

            let data = mem::take(&mut state.data);
            let mut rhai_state = Dynamic::from(data);

            let result = self.resource_man.engine.call_fn_with_options::<Dynamic>(
                rhai_call_options(&mut rhai_state),
                scope,
                ast,
                "handle_transaction",
                (rhai::Map::from([
                    ("coord".into(), Dynamic::from(self.coord)),
                    ("id".into(), Dynamic::from(self.id)),
                    ("source_coord".into(), Dynamic::from(source_coord)),
                    ("source_id".into(), Dynamic::from(source_id)),
                    ("root_coord".into(), Dynamic::from(root_coord)),
                    ("root_id".into(), Dynamic::from(root_id)),
                    ("random".into(), Dynamic::from_int(random())),
                    ("stack".into(), Dynamic::from(stack)),
                ]),),
            );

            state.data = rhai_state.take().cast::<RhaiDataMap>();

            match result {
                Ok(result) => {
                    if let Some(result) = result.try_cast::<TileTransactionResult>() {
                        return self.handle_rhai_transaction_result(state, result);
                    }
                }
                Err(err) => rhai_log_err("handle_transaction", function_id, &err),
            }
        }

        None
    }
}

#[derive(Error, Debug)]
pub enum TileEntityError {
    #[error("the tile id at {0} is no longer existent")]
    NonExistent(TileCoord),
}

#[async_trait::async_trait]
impl Actor for TileEntity {
    type Msg = TileEntityMsg;
    type State = TileEntityState;
    type Arguments = (ActorRef<GameSystemMessage>,);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(TileEntityState::new(args.0))
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            Tick {
                tick_count: _tick_count,
            } => {
                let tile = self
                    .resource_man
                    .registry
                    .tiles
                    .get(&self.id)
                    .ok_or(Box::new(TileEntityError::NonExistent(self.coord)))?;

                if let Some((ast, default_scope, function_id)) = tile
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                {
                    let scope = state
                        .scope
                        .get_or_insert_with(|| default_scope.clone_visible());

                    let data = mem::take(&mut state.data);
                    let mut rhai_state = Dynamic::from(data);

                    let result = self.resource_man.engine.call_fn_with_options::<Dynamic>(
                        rhai_call_options(&mut rhai_state),
                        scope,
                        ast,
                        "handle_tick",
                        (rhai::Map::from([
                            ("coord".into(), Dynamic::from(self.coord)),
                            ("id".into(), Dynamic::from(self.id)),
                            ("random".into(), Dynamic::from_int(random())),
                        ]),),
                    );

                    state.data = rhai_state.take().cast::<RhaiDataMap>();

                    match result {
                        Ok(result) => {
                            if let Some(result) = result.try_cast::<TileResult>() {
                                self.handle_rhai_result(state, result);
                            }
                        }
                        Err(err) => {
                            rhai_log_err("handle_tick", function_id, &err);
                        }
                    }
                }
            }
            Transaction {
                stack,
                source_coord,
                source_id,
                root_coord,
                root_id,
                hidden,
            } => {
                if let Some(record) =
                    self.transaction(state, stack, source_coord, source_id, root_coord, root_id)
                {
                    if !hidden {
                        state.game.send_message(record)?;
                    }
                }
            }
            TransactionResult { result } => {
                let tile = self
                    .resource_man
                    .registry
                    .tiles
                    .get(&self.id)
                    .ok_or(Box::new(TileEntityError::NonExistent(self.coord)))?;

                if let Some((ast, default_scope, function_id)) = tile
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                {
                    let scope = state
                        .scope
                        .get_or_insert_with(|| default_scope.clone_visible());

                    let data = mem::take(&mut state.data);
                    let mut rhai_state = Dynamic::from(data);

                    let result = self.resource_man.engine.call_fn_with_options::<Dynamic>(
                        rhai_call_options(&mut rhai_state),
                        scope,
                        ast,
                        "handle_transaction_result",
                        (rhai::Map::from([
                            ("coord".into(), Dynamic::from(self.coord)),
                            ("id".into(), Dynamic::from(self.id)),
                            ("random".into(), Dynamic::from_int(random())),
                            ("transferred".into(), Dynamic::from(result)),
                        ]),),
                    );

                    state.data = rhai_state.take().cast::<RhaiDataMap>();

                    match result {
                        Ok(_) => {}
                        Err(err) => {
                            rhai_log_err("handle_transaction_result", function_id, &err);
                        }
                    }
                }
            }
            SetData(data) => {
                state.data = RhaiDataMap::from_data_map(data);
            }
            SetDataValue(key, value) => {
                state.data.set(key, value);
            }
            TakeData(reply) => {
                reply.send(mem::take(&mut state.data).to_data_map())?;
            }
            RemoveData(key) => {
                state.data.remove(key);
            }
            GetData(reply) => {
                reply.send(state.data.clone().to_data_map())?;
            }
            GetDataValue(key, reply) => {
                reply.send(state.data.get(key).cloned())?;
            }
            GetDataWithCoord(reply) => {
                reply.send((self.coord, state.data.clone().to_data_map()))?;
            }
            ExtractRequest {
                requested_from_id,
                requested_from_coord,
            } => {
                let tile = self
                    .resource_man
                    .registry
                    .tiles
                    .get(&self.id)
                    .ok_or(Box::new(TileEntityError::NonExistent(self.coord)))?;

                if let Some((ast, default_scope, function_id)) = tile
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                {
                    let scope = state
                        .scope
                        .get_or_insert_with(|| default_scope.clone_visible());

                    let data = mem::take(&mut state.data);
                    let mut rhai_state = Dynamic::from(data);

                    let result = self.resource_man.engine.call_fn_with_options::<Dynamic>(
                        rhai_call_options(&mut rhai_state),
                        scope,
                        ast,
                        "handle_extract_request",
                        (rhai::Map::from([
                            ("coord".into(), Dynamic::from(self.coord)),
                            ("id".into(), Dynamic::from(self.id)),
                            ("random".into(), Dynamic::from_int(random())),
                            (
                                "requested_from_coord".into(),
                                Dynamic::from(requested_from_coord),
                            ),
                            ("requested_from_id".into(), Dynamic::from(requested_from_id)),
                        ]),),
                    );

                    state.data = rhai_state.take().cast::<RhaiDataMap>();

                    match result {
                        Ok(result) => {
                            if let Some(result) = result.try_cast::<TileResult>() {
                                self.handle_rhai_result(state, result);
                            }
                        }
                        Err(err) => rhai_log_err("handle_extract_request", function_id, &err),
                    }
                }
            }
            GetTileConfigUi(reply) => {
                let tile = self
                    .resource_man
                    .registry
                    .tiles
                    .get(&self.id)
                    .ok_or(Box::new(TileEntityError::NonExistent(self.coord)))?;

                if let Some((ast, default_scope, function_id)) = tile
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                {
                    let scope = state
                        .scope
                        .get_or_insert_with(|| default_scope.clone_visible());

                    let data = mem::take(&mut state.data);
                    let mut rhai_state = Dynamic::from(data);

                    let result = self.resource_man.engine.call_fn_with_options::<RhaiUiUnit>(
                        rhai_call_options(&mut rhai_state),
                        scope,
                        ast,
                        "tile_config",
                        (rhai::Map::from([
                            ("coord".into(), Dynamic::from(self.coord)),
                            ("id".into(), Dynamic::from(self.id)),
                        ]),),
                    );

                    state.data = rhai_state.take().cast::<RhaiDataMap>();

                    match result {
                        Ok(result) => {
                            reply.send(Some(result))?;
                        }
                        Err(err) => {
                            rhai_log_err("tile_config", function_id, &err);
                            reply.send(None)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn send_to_tile(
    state: &mut TileEntityState,
    source: TileCoord,
    to: TileCoord,
    msg: TileEntityMsg,
    on_fail: OnFailAction,
) {
    match state
        .game
        .send_message(GameSystemMessage::ForwardMsgToTile {
            source,
            to,
            msg,
            on_fail,
        }) {
        Ok(_) => {}
        Err(_) => {
            state.data = Default::default();
        }
    }
}

fn random() -> i32 {
    thread_rng().next_u32() as i32
}
