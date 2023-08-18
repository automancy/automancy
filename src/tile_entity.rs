use std::mem;
use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use rand::{thread_rng, RngCore};
use rhai::{CallFnOptions, Dynamic, ImmutableString, Scope, INT};

use automancy_defs::coord::TileCoord;
use automancy_defs::id::Id;
use automancy_resources::data::stack::{ItemAmount, ItemStack};
use automancy_resources::data::{Data, DataMap};
use automancy_resources::ResourceManager;

use crate::game::{GameMsg, TickUnit};
use crate::tile_entity::TileEntityMsg::*;

fn rhai_call_options(rhai_state: &mut Dynamic) -> CallFnOptions {
    CallFnOptions::new()
        .eval_ast(false)
        .rewind_scope(true)
        .bind_this_ptr(rhai_state)
}

pub type TileModifier = INT;

#[derive(Debug, Clone)]
pub struct TileEntity {
    /// The ID of the tile entity.
    pub id: Id,
    /// The coordinates of the tile entity.
    pub coord: TileCoord,
    /// The tile modifier of the tile entity.
    pub tile_modifier: TileModifier,
    pub resource_man: Arc<ResourceManager>,
}

pub const RHAI_DATA_MAP_KEY: &str = "data";

/// Represents a tile entity's state. A tile entity is the actor that allows the tile to take, process, and output resources.
#[derive(Debug, Clone)]
pub struct TileEntityState {
    /// A handle to the game.
    game: ActorRef<GameMsg>,

    /// The rhai object map
    rhai_map: rhai::Map, // TODO deprecate the data map entirely

    /// Rhai scope
    scope: Option<Scope<'static>>,

    /// The data map stored by the tile.
    data: DataMap,

    /// Are adjacent tiles requirement fulfilled
    adjacent_fulfilled: bool,
}

impl TileEntityState {
    fn new(game: ActorRef<GameMsg>) -> Self {
        Self {
            game,

            rhai_map: rhai::Map::from([(
                RHAI_DATA_MAP_KEY.into(),
                Dynamic::from(DataMap::default()),
            )]),
            scope: Default::default(),

            data: DataMap::default(),

            adjacent_fulfilled: true,
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
    },
    TransactionResult {
        result: Result<ItemStack, ()>,
    },
    ExtractRequest {
        requested_from_id: Id,
        requested_from_coord: TileCoord,
    },
    AdjacentState {
        fulfilled: bool,
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
                    },
                );
            }
            "consume" => {
                let consumed: ItemAmount = result[1].clone().cast();

                send_to_tile(
                    state,
                    root_coord,
                    TransactionResult {
                        result: Ok(ItemStack {
                            item: stack.item,
                            amount: consumed,
                        }),
                    },
                );

                return Some(GameMsg::RecordTransaction(stack, source_coord, self.coord));
            }
            _ => (),
        }

        None
    }

    fn handle_rhai_result(&self, state: &mut TileEntityState, result: rhai::Array) {
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
        let tile = self.resource_man.registry.tile(self.id).unwrap();

        if let Some((ast, default_scope)) = tile
            .function
            .as_ref()
            .and_then(|v| self.resource_man.functions.get(v))
        {
            let scope = state
                .scope
                .get_or_insert_with(|| default_scope.clone_visible());

            state
                .rhai_map
                .insert(RHAI_DATA_MAP_KEY.into(), Dynamic::from(state.data.clone()));

            let mut rhai_state = Dynamic::from_map(state.rhai_map.clone());

            let options = rhai_call_options(&mut rhai_state);

            let result = self.resource_man.engine.call_fn_with_options::<Dynamic>(
                options,
                scope,
                ast,
                "handle_transaction",
                (rhai::Map::from([
                    (
                        "tile_modifier".into(),
                        Dynamic::from_int(self.tile_modifier),
                    ),
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

            state.rhai_map = rhai_state.take().cast::<rhai::Map>();
            state.data = state
                .rhai_map
                .get(RHAI_DATA_MAP_KEY)
                .cloned()
                .unwrap()
                .cast();

            if let Some(result) = result.ok().and_then(|v| v.try_cast::<rhai::Array>()) {
                return self.handle_rhai_transaction_result(
                    state,
                    stack,
                    source_coord,
                    source_id,
                    root_coord,
                    root_id,
                    result,
                );
            } else {
                send_to_tile(state, source_coord, TransactionResult { result: Err(()) });
            };
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
            Tick { tick_count } => {
                let tile = self.resource_man.registry.tile(self.id).unwrap();

                if tick_count % 10 == 0 {
                    if let Some(script) = state
                        .data
                        .get(&self.resource_man.registry.data_ids.script)
                        .and_then(Data::as_id)
                    {
                        if let Some(script) = self.resource_man.registry.script(*script).cloned() {
                            state
                                .game
                                .send_message(GameMsg::CheckAdjacent {
                                    script,
                                    coord: self.coord,
                                    self_coord: self.coord,
                                })
                                .unwrap();
                        }
                    }
                }

                if !state.adjacent_fulfilled {
                    return Ok(());
                }

                if let Some((ast, default_scope)) = tile
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                {
                    let scope = state
                        .scope
                        .get_or_insert_with(|| default_scope.clone_visible());

                    state
                        .rhai_map
                        .insert(RHAI_DATA_MAP_KEY.into(), Dynamic::from(state.data.clone()));

                    let mut rhai_state = Dynamic::from_map(state.rhai_map.clone());

                    let result = self.resource_man.engine.call_fn_with_options::<Dynamic>(
                        rhai_call_options(&mut rhai_state),
                        scope,
                        ast,
                        "handle_tick",
                        (rhai::Map::from([
                            (
                                "tile_modifier".into(),
                                Dynamic::from_int(self.tile_modifier),
                            ),
                            ("coord".into(), Dynamic::from(self.coord)),
                            ("id".into(), Dynamic::from_int(self.id.into())),
                            ("random".into(), Dynamic::from_int(random())),
                        ]),),
                    );

                    state.rhai_map = rhai_state.take().cast::<rhai::Map>();
                    state.data = state
                        .rhai_map
                        .get(RHAI_DATA_MAP_KEY)
                        .cloned()
                        .unwrap()
                        .cast();

                    if let Some(result) = result.ok().and_then(|v| v.try_cast::<rhai::Array>()) {
                        self.handle_rhai_result(state, result);
                    }
                }
            }
            Transaction {
                stack,
                source_coord,
                source_id,
                root_coord,
                root_id,
            } => {
                if let Some(record) =
                    self.transaction(state, stack, source_coord, source_id, root_coord, root_id)
                {
                    state.game.send_message(record).unwrap();
                }
            }
            TransactionResult { result } => {
                if let Ok(transferred) = result {
                    let tile = self.resource_man.registry.tile(self.id).unwrap();

                    if let Some((ast, default_scope)) = tile
                        .function
                        .as_ref()
                        .and_then(|v| self.resource_man.functions.get(v))
                    {
                        let scope = state
                            .scope
                            .get_or_insert_with(|| default_scope.clone_visible());

                        state
                            .rhai_map
                            .insert(RHAI_DATA_MAP_KEY.into(), Dynamic::from(state.data.clone()));

                        let mut rhai_state = Dynamic::from_map(state.rhai_map.clone());

                        let _result = self.resource_man.engine.call_fn_with_options::<Dynamic>(
                            rhai_call_options(&mut rhai_state),
                            scope,
                            ast,
                            "handle_transaction_result",
                            (rhai::Map::from([
                                (
                                    "tile_modifier".into(),
                                    Dynamic::from_int(self.tile_modifier),
                                ),
                                ("coord".into(), Dynamic::from(self.coord)),
                                ("id".into(), Dynamic::from_int(self.id.into())),
                                ("random".into(), Dynamic::from_int(random())),
                                ("transferred".into(), Dynamic::from(transferred)),
                            ]),),
                        );

                        state.rhai_map = rhai_state.take().cast::<rhai::Map>();
                        state.data = state
                            .rhai_map
                            .get(RHAI_DATA_MAP_KEY)
                            .cloned()
                            .unwrap()
                            .cast();
                    }
                }
            }
            SetData(data) => {
                state.data = data;
            }
            SetDataValue(key, value) => {
                state.data.insert(key, value);
            }
            TakeData(reply) => {
                reply.send(mem::take(&mut state.data)).unwrap();
            }
            GetData(reply) => {
                reply.send(state.data.clone()).unwrap();
            }
            GetDataValue(key, reply) => {
                reply.send(state.data.get(&key).cloned()).unwrap();
            }
            GetDataWithCoord(reply) => {
                reply.send((self.coord, state.data.clone())).unwrap();
            }
            RemoveData(key) => {
                state.data.remove(&key);
            }
            ExtractRequest {
                requested_from_id,
                requested_from_coord,
            } => {
                let tile = self.resource_man.registry.tile(self.id).unwrap();

                if let Some((ast, default_scope)) = tile
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                {
                    let scope = state
                        .scope
                        .get_or_insert_with(|| default_scope.clone_visible());

                    state
                        .rhai_map
                        .insert(RHAI_DATA_MAP_KEY.into(), Dynamic::from(state.data.clone()));

                    let mut rhai_state = Dynamic::from_map(state.rhai_map.clone());

                    let result = self.resource_man.engine.call_fn_with_options::<Dynamic>(
                        rhai_call_options(&mut rhai_state),
                        scope,
                        ast,
                        "handle_extract_request",
                        (rhai::Map::from([
                            (
                                "tile_modifier".into(),
                                Dynamic::from_int(self.tile_modifier),
                            ),
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

                    state.rhai_map = rhai_state.take().cast::<rhai::Map>();
                    state.data = state
                        .rhai_map
                        .get(RHAI_DATA_MAP_KEY)
                        .cloned()
                        .unwrap()
                        .cast();

                    if let Some(result) = result.ok().and_then(|v| v.try_cast::<rhai::Array>()) {
                        self.handle_rhai_result(state, result);
                    }
                }
            }
            AdjacentState { fulfilled } => {
                state.adjacent_fulfilled = fulfilled;
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
            state.data.clear();
        }
    }
}

fn random() -> i32 {
    thread_rng().next_u32() as i32
}
