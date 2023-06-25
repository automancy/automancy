use std::mem;
use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use rand::{thread_rng, RngCore};

use automancy_defs::coord::TileCoord;
use automancy_defs::egui::NumExt;
use automancy_defs::id::Id;
use automancy_defs::log;
use automancy_resources::data::item::item_match;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::ResourceManager;

use crate::game::{GameMsg, TickUnit};
use crate::tile_entity::TileEntityMsg::*;

pub type TileModifier = i32;

pub struct TileEntity {
    /// The ID of the tile entity.
    pub id: Id,
    /// The coordinates of the tile entity.
    pub coord: TileCoord,
    /// The tile modifier of the tile entity.
    pub tile_modifier: TileModifier,
    /// A handle to the game.
    pub game: ActorRef<GameMsg>,
}

/// Represents a tile entity's state. A tile entity is the actor that allows the tile to take, process, and output resources.
#[derive(Debug, Clone)]
pub struct TileEntityState {
    /// The data map stored by the tile.
    data: DataMap,

    /// Are adjacent tiles requirement fulfilled
    adjacent_fulfilled: bool,
}

impl TileEntityState {
    fn new() -> Self {
        Self {
            data: DataMap::default(),

            adjacent_fulfilled: false,
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
        source_type: Option<Id>,
        source_id: Id,
        source_coord: TileCoord,
        source: ActorRef<TileEntityMsg>,
    },
    TransactionResult {
        resource_man: Arc<ResourceManager>,
        result: Result<ItemStack, TransactionError>,
    },
    ExtractRequest {
        resource_man: Arc<ResourceManager>,
        tick_count: TickUnit,
        coord: TileCoord,
        direction: TileCoord,
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
    GetDataValueAndCoord(Id, RpcReplyPort<(TileCoord, Option<Data>)>),
}

/// Represents the various types of errors a tile can run into.
#[derive(Debug, Copy, Clone)]
pub enum TransactionError {
    /// Tried to execute a script but could not find one.
    NoScript,
    /// Transaction type could not be applied to the specified tile.
    NotSuitable,
    /// Target inventory is full.
    Full,
}

struct MachineTickResult {
    pub target: TileCoord,
    pub item_stack: ItemStack,
}

impl TileEntity {
    fn machine_tick(
        &self,
        state: &mut TileEntityState,
        resource_man: &ResourceManager,
    ) -> Option<MachineTickResult> {
        if let Some((target, script)) = state
            .data
            .get(&resource_man.registry.data_ids.target)
            .and_then(Data::as_coord)
            .cloned()
            .zip(
                state
                    .data
                    .get(&resource_man.registry.data_ids.script)
                    .and_then(Data::as_id)
                    .and_then(|id| resource_man.registry.script(*id)),
            )
        {
            let coord = self.coord + target;

            if let Some(inputs) = &script.instructions.inputs {
                if let Some(buffer) = state
                    .data
                    .get_mut(&resource_man.registry.data_ids.buffer)
                    .and_then(Data::as_inventory_mut)
                {
                    for input in inputs {
                        let stored = buffer.get(input.item);

                        if stored < input.amount {
                            return None;
                        }
                    }

                    return Some(MachineTickResult {
                        target: coord,
                        item_stack: script.instructions.output,
                    });
                }
            } else {
                return Some(MachineTickResult {
                    target: coord,
                    item_stack: script.instructions.output,
                });
            }
        }

        None
    }

    fn machine_transaction(
        state: &mut TileEntityState,
        resource_man: &ResourceManager,
        item_stack: ItemStack,
    ) -> Result<ItemStack, TransactionError> {
        let script = state.data.get(&resource_man.registry.data_ids.script);

        if script.is_none() {
            return Err(TransactionError::NoScript);
        }

        if let Some(inputs) = script
            .and_then(Data::as_id)
            .and_then(|id| resource_man.registry.script(*id))
            .and_then(|script| script.instructions.inputs.as_ref())
        {
            if let Data::Inventory(buffer) = state
                .data
                .0
                .entry(resource_man.registry.data_ids.buffer)
                .or_insert_with(Data::inventory)
            {
                let matched = inputs
                    .iter()
                    .find(|v| item_match(&resource_man.registry, item_stack.item.id, v.item.id));

                if matched.is_none() {
                    return Err(TransactionError::NotSuitable);
                }

                let matched = matched.unwrap();

                let amount = buffer.get_mut(item_stack.item);
                if *amount >= matched.amount {
                    return Err(TransactionError::Full);
                }

                *amount += item_stack.amount;
                if *amount > matched.amount {
                    *amount = matched.amount;
                }

                return Ok(*matched);
            }
        }

        Err(TransactionError::NotSuitable)
    }
}

#[async_trait::async_trait]
impl Actor for TileEntity {
    type Msg = TileEntityMsg;
    type State = TileEntityState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(TileEntityState::new())
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
                let tile_type = resource_man.registry.tile(self.id).unwrap().tile_type;

                if tile_type == Some(resource_man.registry.tile_ids.machine) {
                    if tick_count % 10 == 0 {
                        if let Some(script) = state
                            .data
                            .get(&resource_man.registry.data_ids.script)
                            .and_then(Data::as_id)
                        {
                            if let Some(script) = resource_man.registry.script(*script).cloned() {
                                self.game
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

                    if let Some(MachineTickResult { target, item_stack }) =
                        self.machine_tick(state, &resource_man)
                    {
                        send_to_tile(
                            self,
                            state,
                            target,
                            Transaction {
                                resource_man: resource_man.clone(),
                                stack: item_stack,
                                source_type: tile_type,
                                source_id: self.id,
                                source_coord: self.coord,
                                source: myself,
                            },
                            &resource_man,
                        );
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
                            self,
                            state,
                            self.coord + link,
                            ExtractRequest {
                                resource_man: resource_man.clone(),
                                tick_count,
                                coord: self.coord + target,
                                direction: -target,
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
                if self.id == resource_man.registry.tile_ids.void {
                    source
                        .send_message(TransactionResult {
                            resource_man: resource_man.clone(),
                            result: Ok(stack),
                        })
                        .unwrap()
                }

                let tile = resource_man.registry.tile(self.id).unwrap();

                if tile.tile_type == Some(resource_man.registry.tile_ids.machine) {
                    let result = TileEntity::machine_transaction(state, &resource_man, stack);

                    source
                        .send_message(TransactionResult {
                            resource_man: resource_man.clone(),
                            result,
                        })
                        .unwrap();
                }

                if tile.tile_type == Some(resource_man.registry.tile_ids.storage) {
                    if let Some((item, amount)) = state
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
                    {
                        if stack.item == item {
                            let buffer = state
                                .data
                                .0
                                .entry(resource_man.registry.data_ids.buffer)
                                .or_insert_with(Data::inventory)
                                .as_inventory_mut()
                                .unwrap();
                            let stored = buffer.0.entry(item).or_insert(0);

                            if *stored > amount {
                                *stored = amount;
                            }

                            if *stored == amount {
                                source
                                    .send_message(TransactionResult {
                                        resource_man: resource_man.clone(),
                                        result: Err(TransactionError::Full),
                                    })
                                    .unwrap();

                                return Ok(());
                            }

                            let inserting = stack.amount.at_most(amount - *stored);
                            *stored += inserting;

                            source
                                .send_message(TransactionResult {
                                    resource_man: resource_man.clone(),
                                    result: Ok(ItemStack {
                                        item,
                                        amount: inserting,
                                    }),
                                })
                                .unwrap();

                            return Ok(());
                        }
                    }

                    source
                        .send_message(TransactionResult {
                            resource_man: resource_man.clone(),
                            result: Err(TransactionError::NotSuitable),
                        })
                        .unwrap();
                }

                if self.id == resource_man.registry.tile_ids.merger {
                    if let Some(target) = state
                        .data
                        .get(&resource_man.registry.data_ids.target)
                        .and_then(Data::as_coord)
                        .cloned()
                    {
                        let target_coord = self.coord + target;

                        send_to_tile(
                            self,
                            state,
                            target_coord,
                            Transaction {
                                resource_man: resource_man.clone(),
                                stack,
                                source_type: Some(resource_man.registry.tile_ids.transfer),
                                source_id: self.id,
                                source_coord: self.coord,
                                source: source.clone(),
                            },
                            &resource_man,
                        )
                    }
                }

                if self.id == resource_man.registry.tile_ids.splitter {
                    if let Some((a, b, c)) = match self.tile_modifier {
                        0 => Some((
                            TileCoord::TOP_LEFT,
                            TileCoord::BOTTOM_LEFT,
                            TileCoord::RIGHT,
                        )),
                        1 => Some((
                            TileCoord::TOP_RIGHT,
                            TileCoord::BOTTOM_RIGHT,
                            TileCoord::LEFT,
                        )),
                        _ => None,
                    } {
                        let direction = source_coord - self.coord;
                        let (first, second) = if direction == a {
                            (b, c)
                        } else if direction == b {
                            (a, c)
                        } else {
                            (a, b)
                        };

                        let target = if random() % 2 == 0 { first } else { second };

                        let target_coord = self.coord + target;

                        send_to_tile(
                            self,
                            state,
                            target_coord,
                            Transaction {
                                resource_man: resource_man.clone(),
                                stack,
                                source_type: Some(resource_man.registry.tile_ids.transfer),
                                source_id: self.id,
                                source_coord: self.coord,
                                source,
                            },
                            &resource_man,
                        )
                    }
                }
            }
            TransactionResult {
                resource_man,
                result,
            } => {
                if let Ok(transferred) = result {
                    let tile_type = resource_man.registry.tile(self.id).unwrap().tile_type;

                    if tile_type == Some(resource_man.registry.tile_ids.machine) {
                        if let Some(inputs) = state
                            .data
                            .get(&resource_man.registry.data_ids.script)
                            .and_then(Data::as_id)
                            .and_then(|script| resource_man.registry.script(*script))
                            .and_then(|script| script.instructions.inputs.as_ref())
                        {
                            let buffer = state
                                .data
                                .get_mut(&resource_man.registry.data_ids.buffer)
                                .and_then(Data::as_inventory_mut)
                                .unwrap();

                            inputs.iter().for_each(|item_stack| {
                                let stored = buffer.0.entry(item_stack.item).or_insert(0);

                                if *stored < item_stack.amount {
                                    log::error!("in transaction result: tile does not have enough input for the supposed output!");
                                    *stored = 0;
                                } else {
                                    *stored -= item_stack.amount
                                }
                            });
                        }
                    }

                    if tile_type == Some(resource_man.registry.tile_ids.storage) {
                        if let Some(buffer) = state
                            .data
                            .get_mut(&resource_man.registry.data_ids.buffer)
                            .and_then(Data::as_inventory_mut)
                        {
                            buffer.take(transferred.item, transferred.amount);
                        }
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
            GetDataValueAndCoord(key, reply) => {
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
                direction,
            } => {
                let tile_type = resource_man.registry.tile(self.id).unwrap().tile_type;

                if tile_type == Some(resource_man.registry.tile_ids.storage) {
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
                                self,
                                state,
                                coord,
                                Transaction {
                                    resource_man: resource_man.clone(),
                                    stack: ItemStack {
                                        item,
                                        amount: extracting,
                                    },
                                    source_type: tile_type,
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
                            self,
                            state,
                            self.coord + target,
                            ExtractRequest {
                                resource_man: resource_man.clone(),
                                tick_count,
                                coord,
                                direction,
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
    myself: &TileEntity,
    state: &mut TileEntityState,
    coord: TileCoord,
    message: TileEntityMsg,
    resource_man: &ResourceManager,
) {
    match myself
        .game
        .send_message(GameMsg::ForwardMsgToTile(coord, message))
    {
        Ok(_) => {}
        Err(_) => {
            state.data.remove(&resource_man.registry.data_ids.target);
        }
    }
}

fn random() -> u32 {
    thread_rng().next_u32()
}
