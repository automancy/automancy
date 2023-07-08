use std::mem;
use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use rand::{thread_rng, RngCore};
use rhai::{CallFnOptions, Dynamic, ImmutableString, Scope};

use automancy_defs::coord::TileCoord;
use automancy_defs::id::Id;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::ResourceManager;

use crate::game::{GameMsg, TickUnit};
use crate::tile_entity::TileEntityMsg::*;

pub type TileModifier = i32;

#[derive(Debug, Clone)]
pub struct TileEntity {
    /// The ID of the tile entity.
    pub id: Id,
    /// The coordinates of the tile entity.
    pub coord: TileCoord,
    /// The tile modifier of the tile entity.
    pub tile_modifier: TileModifier,
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
        resource_man: Arc<ResourceManager>,
        tick_count: TickUnit,
    },
    Transaction {
        resource_man: Arc<ResourceManager>,
        stack: ItemStack,
        source_id: Id,
        source_coord: TileCoord,
        source: ActorRef<TileEntityMsg>,
    },
    TransactionResult {
        resource_man: Arc<ResourceManager>,
        result: Result<ItemStack, ()>,
    },
    ExtractRequest {
        resource_man: Arc<ResourceManager>,
        tick_count: TickUnit,
        coord: TileCoord,
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
    GetDataValueWithCoord(Id, RpcReplyPort<(TileCoord, Option<Data>)>),
}

impl TileEntity {
    fn transaction(
        &self,
        state: &mut TileEntityState,
        resource_man: Arc<ResourceManager>,
        stack: ItemStack,
        source_coord: TileCoord,
        source: ActorRef<TileEntityMsg>,
    ) -> Option<GameMsg> {
        let tile = resource_man.registry.tile(self.id).unwrap();

        if let Some((ast, default_scope)) = tile
            .function
            .as_ref()
            .and_then(|v| resource_man.functions.get(v))
        {
            let scope = state.scope.get_or_insert_with(|| default_scope.clone());

            state
                .rhai_map
                .insert(RHAI_DATA_MAP_KEY.into(), Dynamic::from(state.data.clone()));

            let mut rhai_state = Dynamic::from_map(state.rhai_map.clone());

            let options = CallFnOptions::new()
                .eval_ast(false)
                .rewind_scope(true)
                .bind_this_ptr(&mut rhai_state);

            let result = resource_man.engine.call_fn_with_options::<Dynamic>(
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
                    ("source_coord".into(), Dynamic::from(source_coord)),
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

            return if let Some(result) = result.ok().and_then(|v| v.try_cast::<rhai::Array>()) {
                let ty: ImmutableString = result[0].clone().cast();

                if ty.as_str() == "pass_on" {
                    let target_coord: TileCoord = result[1].clone().cast();

                    send_to_tile(
                        state,
                        target_coord,
                        Transaction {
                            resource_man: resource_man.clone(),
                            stack,
                            source_id: self.id,
                            source_coord: self.coord,
                            source,
                        },
                        &resource_man,
                    );
                } else if ty.as_str() == "consume" {
                    source
                        .send_message(TransactionResult {
                            resource_man,
                            result: Ok(stack),
                        })
                        .unwrap();
                }

                Some(GameMsg::RecordTransaction(stack, source_coord, self.coord))
            } else {
                source
                    .send_message(TransactionResult {
                        resource_man: resource_man.clone(),
                        result: Err(()),
                    })
                    .unwrap();
                None
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
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            Tick {
                resource_man,
                tick_count,
            } => {
                let tile = resource_man.registry.tile(self.id).unwrap();

                if tick_count % 10 == 0 {
                    if let Some(script) = state
                        .data
                        .get(&resource_man.registry.data_ids.script)
                        .and_then(Data::as_id)
                    {
                        if let Some(script) = resource_man.registry.script(*script).cloned() {
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
                    .and_then(|v| resource_man.functions.get(v))
                {
                    let scope = state.scope.get_or_insert_with(|| default_scope.clone());

                    state
                        .rhai_map
                        .insert(RHAI_DATA_MAP_KEY.into(), Dynamic::from(state.data.clone()));

                    let mut rhai_state = Dynamic::from_map(state.rhai_map.clone());

                    let options = CallFnOptions::new()
                        .eval_ast(false)
                        .rewind_scope(true) //TODO dedupe these
                        .bind_this_ptr(&mut rhai_state);

                    let result = resource_man.engine.call_fn_with_options::<Dynamic>(
                        options,
                        scope,
                        ast,
                        "handle_tick",
                        (rhai::Map::from([
                            (
                                "tile_modifier".into(),
                                Dynamic::from_int(self.tile_modifier),
                            ),
                            ("coord".into(), Dynamic::from(self.coord)),
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
                        let ty: ImmutableString = result[0].clone().cast();

                        if ty.as_str() == "make_transaction" {
                            let target_coord: TileCoord = result[1].clone().cast();
                            let stacks: Vec<ItemStack> = result[2].clone().cast();

                            for stack in stacks {
                                send_to_tile(
                                    state,
                                    target_coord,
                                    Transaction {
                                        resource_man: resource_man.clone(),
                                        stack,
                                        source_id: self.id,
                                        source_coord: self.coord,
                                        source: myself.clone(),
                                    },
                                    &resource_man,
                                );
                            }
                        }
                    }
                }

                if self.id == resource_man.registry.tile_ids.node {
                    if let Some((target, link)) = state
                        .data
                        .get(&resource_man.registry.data_ids.target)
                        .and_then(Data::as_coord)
                        .cloned()
                        .zip(
                            state
                                .data
                                .get(&resource_man.registry.data_ids.link)
                                .and_then(Data::as_coord)
                                .cloned(),
                        )
                    {
                        send_to_tile(
                            state,
                            self.coord + link,
                            ExtractRequest {
                                resource_man: resource_man.clone(),
                                tick_count,
                                coord: self.coord + target,
                            },
                            &resource_man,
                        );
                    }
                }
            }
            Transaction {
                resource_man,
                stack,
                source_coord,
                source,
                ..
            } => {
                if let Some(record) =
                    self.transaction(state, resource_man, stack, source_coord, source)
                {
                    state.game.send_message(record).unwrap();
                }
            }
            TransactionResult {
                resource_man,
                result,
            } => {
                if let Ok(transferred) = result {
                    let tile = resource_man.registry.tile(self.id).unwrap();

                    if let Some((ast, default_scope)) = tile
                        .function
                        .as_ref()
                        .and_then(|v| resource_man.functions.get(v))
                    {
                        let scope = state.scope.get_or_insert_with(|| default_scope.clone());

                        state
                            .rhai_map
                            .insert(RHAI_DATA_MAP_KEY.into(), Dynamic::from(state.data.clone()));

                        let mut rhai_state = Dynamic::from_map(state.rhai_map.clone());

                        let options = CallFnOptions::new()
                            .eval_ast(false)
                            .rewind_scope(true)
                            .bind_this_ptr(&mut rhai_state);

                        let _result = resource_man.engine.call_fn_with_options::<Dynamic>(
                            options,
                            scope,
                            ast,
                            "handle_transaction_result",
                            (rhai::Map::from([
                                (
                                    "tile_modifier".into(),
                                    Dynamic::from_int(self.tile_modifier),
                                ),
                                ("coord".into(), Dynamic::from(self.coord)),
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
                state.data.0.insert(key, value);
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
            GetDataValueWithCoord(key, reply) => {
                reply
                    .send((self.coord, state.data.get(&key).cloned()))
                    .unwrap();
            }
            RemoveData(key) => {
                state.data.remove(&key);
            }
            ExtractRequest {
                resource_man,
                tick_count,
                coord,
            } => {
                if self.id == resource_man.registry.tile_ids.small_storage {
                    if let Some(((item, amount), inventory)) = state
                        .data
                        .get(&resource_man.registry.data_ids.storage)
                        .and_then(Data::as_id)
                        .and_then(|id| resource_man.registry.item(*id).cloned())
                        .zip(
                            state
                                .data
                                .get(&resource_man.registry.data_ids.amount)
                                .and_then(Data::as_amount)
                                .cloned(),
                        )
                        .zip(
                            state
                                .data
                                .get_mut(&resource_man.registry.data_ids.buffer)
                                .and_then(Data::as_inventory_mut),
                        )
                    {
                        let stored = inventory.get(item);
                        let extracting = stored.min(amount);

                        if extracting > 0 {
                            send_to_tile(
                                state,
                                coord,
                                Transaction {
                                    resource_man: resource_man.clone(),
                                    stack: ItemStack {
                                        item,
                                        amount: extracting,
                                    },
                                    source_id: self.id,
                                    source_coord: self.coord,
                                    source: myself,
                                },
                                &resource_man,
                            );
                        }
                    }
                }

                if self.id == resource_man.registry.tile_ids.master_node {
                    if let Some(target) = state
                        .data
                        .get(&resource_man.registry.data_ids.target)
                        .and_then(Data::as_coord)
                        .cloned()
                    {
                        send_to_tile(
                            state,
                            self.coord + target,
                            ExtractRequest {
                                resource_man: resource_man.clone(),
                                tick_count,
                                coord,
                            },
                            &resource_man,
                        );
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

fn send_to_tile(
    state: &mut TileEntityState,
    coord: TileCoord,
    message: TileEntityMsg,
    resource_man: &ResourceManager,
) {
    match state
        .game
        .send_message(GameMsg::ForwardMsgToTile(coord, message))
    {
        Ok(_) => {}
        Err(_) => {
            state.data.remove(&resource_man.registry.data_ids.target);
        }
    }
}

fn random() -> i32 {
    thread_rng().next_u32() as i32
}
