use std::mem;
use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use rand::{thread_rng, RngCore};
use rhai::{CallFnOptions, Dynamic, ImmutableString, Scope, INT};

use automancy_defs::coord::TileCoord;
use automancy_defs::id::Id;
use automancy_defs::log;
use automancy_resources::data::stack::{ItemAmount, ItemStack};
use automancy_resources::data::{Data, DataMap};
use automancy_resources::types::function::RhaiDataMap;
use automancy_resources::ResourceManager;

use crate::game::{GameMsg, TickUnit};
use crate::tile_entity::TileEntityMsg::*;

fn rhai_call_options(rhai_state: &mut Dynamic) -> CallFnOptions {
    CallFnOptions::new()
        .eval_ast(false)
        .rewind_scope(true)
        .bind_this_ptr(rhai_state)
}

fn log_function_err(function_id: &str, err: &rhai::EvalAltResult) {
    match err {
        rhai::EvalAltResult::ErrorFunctionNotFound(..) => {}
        _ => {
            log::error!("In {function_id}: {err}");
        }
    }
}

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
    game: ActorRef<GameMsg>,

    /// The data map stored by the tile.
    data: RhaiDataMap,

    /// Rhai scope
    scope: Option<Scope<'static>>,
}

impl TileEntityState {
    fn new(game: ActorRef<GameMsg>) -> Self {
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
}

impl TileEntity {
    fn handle_rhai_transaction_result(
        &self,
        state: &mut TileEntityState,
        stack: ItemStack,
        source_coord: TileCoord,
        source_id: Id,
        root_coord: TileCoord,
        root_id: Id,
        result: rhai::Array,
    ) -> Option<GameMsg> {
        if result.is_empty() {
            return None;
        }

        let ty: ImmutableString = result[0].clone().cast();

        match ty.as_str() {
            "pass_on" => {
                let coord: TileCoord = result[1].clone().cast();

                send_to_tile(
                    state,
                    coord,
                    Transaction {
                        stack,
                        source_id: self.id,
                        source_coord: self.coord,
                        root_id,
                        root_coord,
                        hidden: false,
                    },
                );

                return Some(GameMsg::RecordTransaction(stack, source_coord, self.coord));
            }
            "proxy" => {
                let coord: TileCoord = result[1].clone().cast();

                send_to_tile(
                    state,
                    coord,
                    Transaction {
                        stack,
                        source_id,
                        source_coord,
                        root_id,
                        root_coord,
                        hidden: false,
                    },
                );

                return Some(GameMsg::RecordTransaction(stack, self.coord, coord));
            }
            "consume" => {
                let consumed: ItemAmount = result[1].clone().cast();

                send_to_tile(
                    state,
                    root_coord,
                    TransactionResult {
                        result: ItemStack {
                            item: stack.item,
                            amount: consumed,
                        },
                    },
                );

                return Some(GameMsg::RecordTransaction(stack, source_coord, self.coord));
            }
            _ => (),
        }

        None
    }

    fn handle_rhai_result(&self, state: &mut TileEntityState, result: rhai::Array) {
        if result.is_empty() {
            return;
        }

        let ty: ImmutableString = result[0].clone().cast();

        match ty.as_str() {
            "make_transaction" => {
                let coord: TileCoord = result[1].clone().cast();
                let source_id: Id = result[2].clone().cast::<INT>().into();
                let source_coord: TileCoord = result[3].clone().cast();

                let stacks: Vec<ItemStack> = result[4]
                    .clone()
                    .try_cast()
                    .unwrap_or_else(|| result[4].clone().into_typed_array().unwrap());

                for stack in stacks {
                    send_to_tile(
                        state,
                        coord,
                        Transaction {
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
            "make_hidden_transaction" => {
                let coord: TileCoord = result[1].clone().cast();
                let source_id: Id = result[2].clone().cast::<INT>().into();
                let source_coord: TileCoord = result[3].clone().cast();

                let stacks: Vec<ItemStack> = result[4]
                    .clone()
                    .try_cast()
                    .unwrap_or_else(|| result[4].clone().into_typed_array().unwrap());

                for stack in stacks {
                    send_to_tile(
                        state,
                        coord,
                        Transaction {
                            stack,
                            source_coord,
                            source_id,
                            root_coord: source_coord,
                            root_id: source_id,
                            hidden: true,
                        },
                    );
                }
            }
            "make_extract_request" => {
                let coord: TileCoord = result[1].clone().cast();
                let requested_from_id: Id = result[2].clone().cast::<INT>().into();
                let requested_from_coord: TileCoord = result[3].clone().cast();

                send_to_tile(
                    state,
                    coord,
                    ExtractRequest {
                        requested_from_id,
                        requested_from_coord,
                    },
                );
            }
            _ => (),
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
    ) -> Option<GameMsg> {
        let tile = self.resource_man.registry.tiles.get(&self.id).unwrap();

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
                    ("id".into(), Dynamic::from_int(self.id.into())),
                    ("source_coord".into(), Dynamic::from(source_coord)),
                    ("source_id".into(), Dynamic::from_int(source_id.into())),
                    ("root_coord".into(), Dynamic::from(root_coord)),
                    ("root_id".into(), Dynamic::from_int(root_id.into())),
                    ("random".into(), Dynamic::from_int(random())),
                    ("stack".into(), Dynamic::from(stack)),
                ]),),
            );

            state.data = rhai_state.take().cast::<RhaiDataMap>();

            match result {
                Ok(result) => {
                    if let Some(result) = result.try_cast::<rhai::Array>() {
                        return self.handle_rhai_transaction_result(
                            state,
                            stack,
                            source_coord,
                            source_id,
                            root_coord,
                            root_id,
                            result,
                        );
                    }
                }
                Err(err) => log_function_err(function_id, &err),
            }
        }

        None
    }
}

#[async_trait::async_trait]
impl Actor for TileEntity {
    type Msg = TileEntityMsg;
    type State = TileEntityState;
    type Arguments = (ActorRef<GameMsg>,);

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
                let tile = self.resource_man.registry.tiles.get(&self.id).unwrap();

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
                            ("id".into(), Dynamic::from_int(self.id.into())),
                            ("random".into(), Dynamic::from_int(random())),
                        ]),),
                    );

                    state.data = rhai_state.take().cast::<RhaiDataMap>();

                    match result {
                        Ok(result) => {
                            if let Some(result) = result.try_cast::<rhai::Array>() {
                                self.handle_rhai_result(state, result);
                            }
                        }
                        Err(err) => {
                            log_function_err(function_id, &err);
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
                        state.game.send_message(record).unwrap();
                    }
                }
            }
            TransactionResult { result } => {
                let tile = self.resource_man.registry.tiles.get(&self.id).unwrap();

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
                            ("id".into(), Dynamic::from_int(self.id.into())),
                            ("random".into(), Dynamic::from_int(random())),
                            ("transferred".into(), Dynamic::from(result)),
                        ]),),
                    );

                    state.data = rhai_state.take().cast::<RhaiDataMap>();

                    match result {
                        Ok(_) => {}
                        Err(err) => {
                            log_function_err(function_id, &err);
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
                reply
                    .send(mem::take(&mut state.data).to_data_map())
                    .unwrap();
            }
            GetData(reply) => {
                reply.send(state.data.clone().to_data_map()).unwrap();
            }
            GetDataValue(key, reply) => {
                reply.send(state.data.get(key)).unwrap();
            }
            GetDataWithCoord(reply) => {
                reply
                    .send((self.coord, state.data.clone().to_data_map()))
                    .unwrap();
            }
            RemoveData(key) => {
                state.data.remove(key);
            }
            ExtractRequest {
                requested_from_id,
                requested_from_coord,
            } => {
                let tile = self.resource_man.registry.tiles.get(&self.id).unwrap();

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
                            ("id".into(), Dynamic::from_int(self.id.into())),
                            ("random".into(), Dynamic::from_int(random())),
                            (
                                "requested_from_coord".into(),
                                Dynamic::from(requested_from_coord),
                            ),
                            (
                                "requested_from_id".into(),
                                Dynamic::from_int(requested_from_id.into()),
                            ),
                        ]),),
                    );

                    state.data = rhai_state.take().cast::<RhaiDataMap>();

                    match result {
                        Ok(result) => {
                            if let Some(result) = result.try_cast::<rhai::Array>() {
                                self.handle_rhai_result(state, result);
                            }
                        }
                        Err(err) => log_function_err(function_id, &err),
                    }
                }
            }
        }

        Ok(())
    }
}

fn send_to_tile(state: &mut TileEntityState, coord: TileCoord, message: TileEntityMsg) {
    match state
        .game
        .send_message(GameMsg::ForwardMsgToTile(coord, message))
    {
        Ok(_) => {}
        Err(_) => {
            state.data = Default::default();
        }
    }
}

fn random() -> i32 {
    thread_rng().next_u32() as i32
}
