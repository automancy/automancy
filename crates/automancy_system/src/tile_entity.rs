use std::{mem, sync::Arc};

use automancy_defs::{
    coord::TileCoord,
    id::{Id, TileId},
    stack::ItemStack,
};
use automancy_resources::{
    FunctionInfo, ResourceManager,
    data::{Data, DataMap},
    rhai_call_options, rhai_log_err,
    rhai_render::RenderCommand,
    rhai_ui::RhaiUiUnit,
    types::function::{OnFailAction, TileResult, TileTransactionResult},
};
use hashbrown::HashSet;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use rand::{RngCore, rng};
use rhai::{Dynamic, Scope};
use thiserror::Error;

use crate::{
    game::{GameSystemMessage, TickUnit},
    tile_entity::TileEntityMsg::*,
};

pub type TileEntityWithId = (TileId, ActorRef<TileEntityMsg>);

fn run_tile_function<Result: 'static, const SIZE: usize>(
    resource_man: &ResourceManager,
    id: TileId,
    coord: TileCoord,
    data: &mut DataMap,
    field_changes: &mut HashSet<Id>,
    (ast, metadata): &FunctionInfo,
    args: [(&'static str, Dynamic); SIZE],
    function: &'static str,
) -> Option<Result> {
    let tile_def = resource_man.registry.tiles.get(&id)?;
    let mut rhai_state = Dynamic::from(data.clone());

    let mut input = rhai::Map::from([
        ("coord".into(), Dynamic::from(coord)),
        ("id".into(), Dynamic::from(id)),
        ("random".into(), Dynamic::from_int(random())),
        ("setup".into(), Dynamic::from(tile_def.data.clone())),
    ]);

    input.extend(args.into_iter().map(|(k, v)| (k.into(), v)));

    let result = resource_man.engine.call_fn_with_options::<Dynamic>(
        rhai_call_options(&mut rhai_state),
        &mut Scope::new(),
        ast,
        function,
        (input,),
    );

    {
        let new_data = rhai_state.cast::<DataMap>();

        for k in new_data.keys().cloned() {
            if data.get(k).is_none() {
                field_changes.insert(k);
            }
        }

        for (k, v) in mem::take(data) {
            let new_v = new_data.get(k);
            if new_v.is_none() || new_v.is_some_and(|new| new != &v) {
                field_changes.insert(k);
            }
        }

        *data = new_data;
    }

    match result {
        Ok(result) => result.try_cast::<Result>(),
        Err(err) => {
            rhai_log_err(function, &metadata.str_id, &err, Some(coord));
            None
        }
    }
}

pub fn collect_render_commands(
    resource_man: &ResourceManager,
    id: TileId,
    coord: TileCoord,
    data: &mut DataMap,
    field_changes: &mut HashSet<Id>,
    loading: bool,
    unloading: bool,
) -> Option<Vec<RenderCommand>> {
    let tile_def = resource_man.registry.tiles.get(&id)?;

    if let Some(function @ (_, metadata)) = tile_def
        .function
        .as_ref()
        .and_then(|v| resource_man.functions.get(v))
    {
        if !loading
            && !unloading
            && !metadata
                .render_listening_to_fields
                .iter()
                .all(|v| field_changes.contains(v))
        {
            return None;
        }

        let last_changes = mem::take(field_changes);

        if let Some(result) = run_tile_function(
            resource_man,
            id,
            coord,
            data,
            field_changes,
            function,
            [
                ("field_changes", Dynamic::from_iter(last_changes)),
                ("loading", Dynamic::from_bool(loading)),
                ("unloading", Dynamic::from_bool(unloading)),
            ],
            "tile_render",
        ) as Option<rhai::Array>
        {
            return Some(
                result
                    .into_iter()
                    .flat_map(|v| v.try_cast::<RenderCommand>())
                    .collect::<Vec<_>>(),
            );
        }
    }

    None
}

#[derive(Debug, Clone)]
pub struct TileEntity {
    /// The ID of the tile entity.
    pub id: TileId,
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
    data: DataMap,

    /// The field changed since last render request.
    field_changes: HashSet<Id>,
}

impl TileEntityState {
    fn new(game: ActorRef<GameSystemMessage>) -> Self {
        Self {
            game,

            data: Default::default(),

            field_changes: HashSet::new(),
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
        source_id: TileId,
        root_coord: TileCoord,
        root_id: TileId,
        hidden: bool,
    },
    TransactionResult {
        result: ItemStack,
    },
    ExtractRequest {
        requested_from_id: TileId,
        requested_from_coord: TileCoord,
    },
    CollectRenderCommands {
        reply: RpcReplyPort<Option<Vec<RenderCommand>>>,
        loading: bool,
        unloading: bool,
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

                None
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

                None
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
        source_id: TileId,
        root_coord: TileCoord,
        root_id: TileId,
    ) -> Option<GameSystemMessage> {
        let tile = self.resource_man.registry.tiles.get(&self.id)?;

        if let Some(function) = tile
            .function
            .as_ref()
            .and_then(|v| self.resource_man.functions.get(v))
            && let Some(result) = run_tile_function(
                &self.resource_man,
                self.id,
                self.coord,
                &mut state.data,
                &mut state.field_changes,
                function,
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
}

#[derive(Error, Debug)]
pub enum TileEntityError {
    #[error("the tile ID at {0} is no longer existent")]
    NonExistent(TileCoord),
}

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
                let tile_def = self
                    .resource_man
                    .registry
                    .tiles
                    .get(&self.id)
                    .ok_or(Box::new(TileEntityError::NonExistent(self.coord)))?;

                if let Some(function) = tile_def
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                    && let Some(result) = run_tile_function(
                        &self.resource_man,
                        self.id,
                        self.coord,
                        &mut state.data,
                        &mut state.field_changes,
                        function,
                        [],
                        "handle_tick",
                    )
                {
                    self.handle_rhai_result(state, result);
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
                    && !hidden
                {
                    state.game.send_message(record)?;
                }
            }
            TransactionResult { result } => {
                let tile_def = self
                    .resource_man
                    .registry
                    .tiles
                    .get(&self.id)
                    .ok_or(Box::new(TileEntityError::NonExistent(self.coord)))?;

                if let Some(function) = tile_def
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                {
                    let _: Option<()> = run_tile_function(
                        &self.resource_man,
                        self.id,
                        self.coord,
                        &mut state.data,
                        &mut state.field_changes,
                        function,
                        [("transferred", Dynamic::from(result))],
                        "handle_transaction_result",
                    );
                }
            }
            ExtractRequest {
                requested_from_id,
                requested_from_coord,
            } => {
                let tile_def = self
                    .resource_man
                    .registry
                    .tiles
                    .get(&self.id)
                    .ok_or(Box::new(TileEntityError::NonExistent(self.coord)))?;

                if let Some(function) = tile_def
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                    && let Some(result) = run_tile_function(
                        &self.resource_man,
                        self.id,
                        self.coord,
                        &mut state.data,
                        &mut state.field_changes,
                        function,
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
            GetTileConfigUi(reply) => {
                let tile_def = self
                    .resource_man
                    .registry
                    .tiles
                    .get(&self.id)
                    .ok_or(Box::new(TileEntityError::NonExistent(self.coord)))?;

                if let Some(function) = tile_def
                    .function
                    .as_ref()
                    .and_then(|v| self.resource_man.functions.get(v))
                {
                    if let Some(result) = run_tile_function(
                        &self.resource_man,
                        self.id,
                        self.coord,
                        &mut state.data,
                        &mut state.field_changes,
                        function,
                        [],
                        "tile_config",
                    ) {
                        reply.send(Some(result))?;
                    } else {
                        reply.send(None)?;
                    }
                }
            }
            CollectRenderCommands {
                reply,
                loading,
                unloading,
            } => {
                reply.send(collect_render_commands(
                    &self.resource_man,
                    self.id,
                    self.coord,
                    &mut state.data,
                    &mut state.field_changes,
                    loading,
                    unloading,
                ))?;
            }
            SetData(data) => {
                state.field_changes.extend(data.keys());
                state.data = data;
            }
            SetDataValue(key, value) => {
                state.field_changes.insert(key);
                state.data.set(key, value);
            }
            TakeData(reply) => {
                state.field_changes.extend(state.data.keys());
                reply.send(mem::take(&mut state.data))?;
            }
            RemoveData(key) => {
                state.field_changes.insert(key);
                state.data.remove(key);
            }
            GetData(reply) => {
                reply.send(state.data.clone())?;
            }
            GetDataValue(key, reply) => {
                reply.send(state.data.get(key).cloned())?;
            }
            GetDataWithCoord(reply) => {
                reply.send((self.coord, state.data.clone()))?;
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
    rng().next_u32() as i32
}
