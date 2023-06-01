use std::sync::Arc;

use egui::NumExt;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use rand::{thread_rng, RngCore};
use rune::{Any, ContextError, Module, Vm};
use serde::{Deserialize, Serialize};

use crate::game::item::{ItemAmount, ItemStack};
use crate::game::tile::coord::TileCoord;
use crate::game::tile::entity::TileEntityMsg::*;
use crate::game::tile::entity::TransactionError::*;
use crate::game::tile::inventory::{Inventory, InventoryRaw};
use crate::game::{GameMsg, TickUnit};
use crate::resource::tile::TileType;
use crate::resource::tile::TileType::*;
use crate::resource::ResourceManager;
use crate::util::id::{Id, IdRaw, Interner};

/// Represents the various types of errors a tile can run into.
#[derive(Debug, Copy, Clone, Any)]
pub enum TransactionError {
    /// Tried to execute a script but could not find one.
    #[rune(constructor)]
    NoScript,
    /// Transaction type could not be applied to the specified tile.
    #[rune(constructor)]
    NotSuitable,
    /// Target inventory is full.
    #[rune(constructor)]
    Full,
}

/// Represents the data a tile entity holds. This data is given to Rune functions.
#[derive(Debug, Clone, Any)]
pub enum Data {
    /// The tile entity's inventory.
    #[rune(constructor)]
    Inventory(#[rune(get)] Inventory),
    /// The coordinates of the tile.
    #[rune(constructor)]
    Coord(#[rune(get, copy)] TileCoord),
    /// The tile's ID.
    #[rune(constructor)]
    VecCoord(#[rune(get)] Vec<TileCoord>),

    #[rune(constructor)]
    Bool(#[rune(get)] bool),

    #[rune(constructor)]
    Id(#[rune(get, copy)] Id),

    #[rune(constructor)]
    Amount(#[rune(get)] ItemAmount),
}

impl Data {
    /// Gets the default Inventory.
    pub fn inventory() -> Self {
        Self::Inventory(Default::default())
    }

    pub fn vec_coord() -> Self {
        Self::VecCoord(Default::default())
    }

    /// Gets a mutable reference to  the tile's Inventory, or None.
    pub fn as_inventory_mut(&mut self) -> Option<&mut Inventory> {
        if let Self::Inventory(inventory) = self {
            return Some(inventory);
        }
        None
    }

    /// Gets a mutable reference to  the tile's coordinates, or None.
    pub fn as_coord_mut(&mut self) -> Option<&mut TileCoord> {
        if let Self::Coord(coord) = self {
            return Some(coord);
        }
        None
    }

    pub fn as_bool_mut(&mut self) -> Option<&mut bool> {
        if let Self::Bool(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_vec_coord_mut(&mut self) -> Option<&mut Vec<TileCoord>> {
        if let Self::VecCoord(vec) = self {
            return Some(vec);
        }
        None
    }

    /// Gets a mutable reference to the tile's ID, or None.
    pub fn as_id_mut(&mut self) -> Option<&mut Id> {
        if let Self::Id(id) = self {
            return Some(id);
        }
        None
    }

    pub fn as_amount_mut(&mut self) -> Option<&mut ItemAmount> {
        if let Self::Amount(amount) = self {
            return Some(amount);
        }
        None
    }

    /// Gets an immutable reference to  the tile's Inventory, or None.
    pub fn as_inventory(&self) -> Option<&Inventory> {
        if let Self::Inventory(inventory) = self {
            return Some(inventory);
        }
        None
    }

    /// Gets an immutable reference to  the tile's coordinates, or None.
    pub fn as_coord(&self) -> Option<&TileCoord> {
        if let Self::Coord(coord) = self {
            return Some(coord);
        }
        None
    }

    /// Gets an immutable reference to the tile's ID, or None.
    pub fn as_bool(&self) -> Option<&bool> {
        if let Self::Bool(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_vec_coord(&self) -> Option<&Vec<TileCoord>> {
        if let Self::VecCoord(vec) = self {
            return Some(vec);
        }
        None
    }

    pub fn as_id(&self) -> Option<&Id> {
        if let Self::Id(id) = self {
            return Some(id);
        }
        None
    }

    pub fn as_amount(&self) -> Option<&ItemAmount> {
        if let Self::Amount(amount) = self {
            return Some(amount);
        }
        None
    }
}

pub type DataMap = std::collections::HashMap<String, Data>;

#[derive(Debug, Serialize, Deserialize)]
pub enum DataRaw {
    Inventory(InventoryRaw),
    Coord(TileCoord),
    Bool(bool),
    VecCoord(Vec<TileCoord>),
    Id(IdRaw),
    Amount(ItemAmount),
}

pub type DataMapRaw = std::collections::HashMap<String, DataRaw>;

pub fn data_to_raw(data: DataMap, interner: &Interner) -> DataMapRaw {
    data.into_iter()
        .map(|(key, value)| {
            let value = match value {
                Data::Inventory(inventory) => {
                    DataRaw::Inventory(InventoryRaw::from_inventory(inventory, interner))
                }
                Data::Coord(coord) => DataRaw::Coord(coord),
                Data::Id(id) => DataRaw::Id(IdRaw::parse(interner.resolve(id).unwrap())),
                Data::Amount(amount) => DataRaw::Amount(amount),
                Data::VecCoord(vec) => DataRaw::VecCoord(vec),
                Data::Bool(v) => DataRaw::Bool(v),
            };

            (key, value)
        })
        .collect()
}

pub fn data_from_raw(data: DataMapRaw, interner: &Interner) -> DataMap {
    data.into_iter()
        .map(|(key, value)| {
            let value = match value {
                DataRaw::Inventory(inventory) => Data::Inventory(inventory.to_inventory(interner)),
                DataRaw::Coord(coord) => Data::Coord(coord),
                DataRaw::Id(id) => Data::Id(interner.get(id.to_string()).unwrap()),
                DataRaw::Amount(amount) => Data::Amount(amount),
                DataRaw::VecCoord(vec) => Data::VecCoord(vec),
                DataRaw::Bool(v) => Data::Bool(v),
            };

            (key, value)
        })
        .collect()
}

/// Represents a tile entity. A tile entity is the actor that allows the tile to take, process, and output resources.
#[derive(Debug, Clone, Any)]
pub struct TileEntityState {
    /// The ID of the tile entity.
    #[rune(get, copy)]
    pub id: Id,
    /// The coordinates of the tile entity.
    #[rune(get, copy)]
    pub coord: TileCoord,
    /// The tile state of the tile entity.
    #[rune(get, copy)]
    pub tile_modifier: TileModifier,
    /// The data map stored by the tile.
    #[rune(get, set)] // TODO use methods
    pub data: DataMap,

    tick_pause: bool,
    adjacent_fulfilled: bool,
}

impl TileEntityState {
    /// Creates a new tile entity.
    fn new(id: Id, coord: TileCoord, tile_modifier: TileModifier) -> Self {
        Self {
            id,
            coord,
            tile_modifier,
            data: DataMap::default(),

            tick_pause: false,
            adjacent_fulfilled: false,
        }
    }
    /// Adds tile entities to the Rune API.
    pub fn install(module: &mut Module) -> Result<(), ContextError> {
        module.ty::<Data>()?;
        module.inst_fn("clone", Data::clone)?;
        module.function(&["Data", "inventory"], Data::inventory)?;

        module.ty::<TransactionError>()?;

        module.ty::<Inventory>()?;
        module.inst_fn("get", Inventory::get)?;
        module.inst_fn("insert", Inventory::insert)?;

        module.ty::<Self>()?;

        Ok(())
    }
}

pub type TileModifier = i32;

#[derive(Debug)]
pub enum TileEntityMsg {
    Tick {
        resource_man: Arc<ResourceManager>,
        tick_count: TickUnit,
    },
    Transaction {
        resource_man: Arc<ResourceManager>,
        tick_count: TickUnit,
        item_stack: ItemStack,
        source_type: TileType,
        direction: TileCoord,
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
    SetData(String, Data),
    RemoveData(&'static str),
    GetData(RpcReplyPort<DataMap>),
    GetDataValue(&'static str, RpcReplyPort<Option<Data>>),
    GetDataValueAndCoord(&'static str, RpcReplyPort<(TileCoord, Option<Data>)>),
}

pub struct TileEntity {
    pub(crate) game: ActorRef<GameMsg>,
}

#[async_trait::async_trait]
impl Actor for TileEntity {
    type Msg = TileEntityMsg;
    type State = TileEntityState;
    type Arguments = (Id, TileCoord, TileModifier);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(TileEntityState::new(args.0, args.1, args.2))
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
                state.tick_pause = false;

                let tile_type = &resource_man.registry.get_tile(&state.id).unwrap().tile_type;

                match tile_type {
                    Machine(_) => {
                        if tick_count % 10 == 0 {
                            if let Some(script) = state.data.get("script").and_then(Data::as_id) {
                                if let Some(script) = resource_man.registry.get_script(script) {
                                    self.game
                                        .send_message(GameMsg::CheckAdjacent {
                                            script,
                                            coord: state.coord,
                                            self_coord: state.coord,
                                        })
                                        .unwrap();
                                }
                            }
                        }

                        if !state.adjacent_fulfilled {
                            return Ok(());
                        }

                        let function =
                            resource_man.functions[&resource_man.registry.tile_ids.machine].clone();
                        let mut vm = Vm::new(function.context, function.unit);

                        let output = vm
                            .call(
                                ["handle_tick"],
                                (state.clone(), &resource_man.registry, tile_type, tick_count),
                            )
                            .unwrap();

                        if let Ok(output) = output.into_tuple() {
                            let output = output.take().unwrap();

                            let target_coord: TileCoord = output
                                .get(0)
                                .unwrap()
                                .clone()
                                .into_any()
                                .unwrap()
                                .take_downcast()
                                .unwrap();

                            let direction: TileCoord = output
                                .get(1)
                                .unwrap()
                                .clone()
                                .into_any()
                                .unwrap()
                                .take_downcast()
                                .unwrap();

                            let item_stack: ItemStack = output
                                .get(2)
                                .unwrap()
                                .clone()
                                .into_any()
                                .unwrap()
                                .take_downcast()
                                .unwrap();

                            send_to_tile_coord(
                                self,
                                state,
                                target_coord,
                                Transaction {
                                    resource_man: resource_man.clone(),
                                    tick_count,
                                    item_stack,
                                    source_type: tile_type.clone(),
                                    direction,
                                    source: myself,
                                },
                            );
                        }
                    }
                    Transfer(id) => {
                        if id == &resource_man.registry.tile_ids.node {
                            if let Some((target, link)) = state
                                .data
                                .get("target")
                                .and_then(Data::as_coord)
                                .cloned()
                                .zip(state.data.get("link").and_then(Data::as_coord).cloned())
                            {
                                send_to_tile_coord(
                                    self,
                                    state,
                                    state.coord + link,
                                    ExtractRequest {
                                        resource_man,
                                        tick_count,
                                        coord: state.coord + target,
                                        direction: -target,
                                    },
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            Transaction {
                resource_man,
                tick_count,
                item_stack,
                source_type,
                direction,
                source,
            } => {
                let tile_type = resource_man.registry.get_tile(&state.id).unwrap().tile_type;

                match &tile_type {
                    Machine(_) => {
                        let function =
                            resource_man.functions[&resource_man.registry.tile_ids.machine].clone();
                        let mut vm = Vm::new(function.context, function.unit);

                        let output = vm
                            .call(
                                ["handle_transaction"],
                                (
                                    state,
                                    &resource_man.registry,
                                    tick_count,
                                    item_stack,
                                    &source_type,
                                    direction,
                                    random(),
                                ),
                            )
                            .unwrap();

                        if let Ok(output) = output.into_result() {
                            let result: Result<ItemStack, TransactionError> = output
                                .take()
                                .unwrap()
                                .map(|v| {
                                    v.into_any().unwrap().take_downcast::<ItemStack>().unwrap()
                                })
                                .map_err(|v| {
                                    v.into_any()
                                        .unwrap()
                                        .take_downcast::<TransactionError>()
                                        .unwrap()
                                });

                            source
                                .send_message(TransactionResult {
                                    result,
                                    resource_man,
                                })
                                .unwrap();
                        }
                    }
                    Transfer(id) => {
                        if let Some(function) = resource_man.functions.get(id).cloned() {
                            let mut vm = Vm::new(function.context, function.unit);

                            let output = vm
                                .call(
                                    ["handle_transaction"],
                                    (
                                        state.clone(),
                                        &resource_man.registry,
                                        tick_count,
                                        item_stack,
                                        &source_type,
                                        direction,
                                        random(),
                                    ),
                                )
                                .unwrap();

                            if let Ok(output) = output.into_tuple() {
                                let output = output.take().unwrap();

                                let target_coord: TileCoord = output
                                    .get(0)
                                    .unwrap()
                                    .clone()
                                    .into_any()
                                    .unwrap()
                                    .take_downcast()
                                    .unwrap();

                                let item_stack: ItemStack = output
                                    .get(1)
                                    .unwrap()
                                    .clone()
                                    .into_any()
                                    .unwrap()
                                    .take_downcast()
                                    .unwrap();

                                let target: TileCoord = output
                                    .get(2)
                                    .unwrap()
                                    .clone()
                                    .into_any()
                                    .unwrap()
                                    .take_downcast()
                                    .unwrap();

                                send_to_tile_coord(
                                    self,
                                    state,
                                    target_coord,
                                    Transaction {
                                        resource_man,
                                        tick_count,
                                        item_stack,
                                        source_type: tile_type,
                                        direction: -target,
                                        source,
                                    },
                                );
                            }
                        }
                    }
                    Void => {
                        source
                            .send_message(TransactionResult {
                                resource_man,
                                result: Ok(item_stack),
                            })
                            .unwrap();
                    }
                    Storage(_) => {
                        if let Some((item, amount)) = state
                            .data
                            .get("storage")
                            .and_then(Data::as_id)
                            .and_then(|id| resource_man.registry.get_item(id))
                            .zip(state.data.get("amount").and_then(Data::as_amount).cloned())
                        {
                            if item_stack.item == item {
                                let buffer = state
                                    .data
                                    .entry("buffer".to_owned())
                                    .or_insert_with(Data::inventory)
                                    .as_inventory_mut()
                                    .unwrap();
                                let stored = buffer.0.entry(item.id).or_insert(0);

                                if *stored > amount {
                                    *stored = amount;
                                }

                                if *stored == amount {
                                    source
                                        .send_message(TransactionResult {
                                            resource_man,
                                            result: Err(Full),
                                        })
                                        .unwrap();

                                    return Ok(());
                                }

                                let inserting = item_stack.amount.at_most(amount - *stored);
                                *stored += inserting;

                                source
                                    .send_message(TransactionResult {
                                        resource_man,
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
                                resource_man,
                                result: Err(NotSuitable),
                            })
                            .unwrap();
                    }
                    _ => {
                        source
                            .send_message(TransactionResult {
                                resource_man,
                                result: Err(NotSuitable),
                            })
                            .unwrap();
                    }
                }
            }
            TransactionResult {
                resource_man,
                result,
            } => {
                if let Ok(transferred) = result {
                    let tile_type = &resource_man.registry.get_tile(&state.id).unwrap().tile_type;

                    match tile_type {
                        Machine(_) => {
                            if let Some(inputs) = state
                                .data
                                .get("script")
                                .and_then(Data::as_id)
                                .and_then(|script| resource_man.registry.get_script(script))
                                .and_then(|script| script.instructions.inputs)
                            {
                                let buffer = state
                                    .data
                                    .get_mut("buffer")
                                    .and_then(Data::as_inventory_mut)
                                    .unwrap();

                                inputs.iter().for_each(|item_stack| {
                                    let stored = buffer.0.entry(item_stack.item.id).or_insert(0);

                                    if *stored < item_stack.amount {
                                        log::error!("in transaction result: tile does not have enough input for the supposed output!");
                                        *stored = 0;
                                    } else {
                                        *stored -= item_stack.amount
                                    }
                                });
                            }
                        }
                        Storage(_) => {
                            if let Some(buffer) = state
                                .data
                                .get_mut("buffer")
                                .and_then(Data::as_inventory_mut)
                            {
                                buffer.take(transferred.item.id, transferred.amount);
                            }
                        }
                        _ => {}
                    }
                }
            }
            SetData(key, value) => {
                state.data.insert(key, value);
            }
            GetData(reply) => {
                reply.send(state.data.clone()).unwrap();
            }
            GetDataValue(key, reply) => {
                reply.send(state.data.get(key).cloned()).unwrap();
            }
            GetDataValueAndCoord(key, reply) => {
                reply
                    .send((state.coord, state.data.get(key).cloned()))
                    .unwrap();
            }
            RemoveData(key) => {
                state.data.remove(key);
            }
            ExtractRequest {
                resource_man,
                tick_count,
                coord,
                direction,
            } => {
                if state.tick_pause {
                    return Ok(());
                }

                let tile_type = &resource_man.registry.get_tile(&state.id).unwrap().tile_type;

                match tile_type {
                    Storage(_) => {
                        if let Some(((item, amount), inventory)) = state
                            .data
                            .get("storage")
                            .and_then(Data::as_id)
                            .and_then(|id| resource_man.registry.get_item(id))
                            .zip(state.data.get("amount").and_then(Data::as_amount).cloned())
                            .zip(
                                state
                                    .data
                                    .get_mut("buffer")
                                    .and_then(Data::as_inventory_mut),
                            )
                        {
                            let stored = inventory.get(item.id);
                            let extracting = stored.min(amount);

                            if extracting > 0 {
                                send_to_tile_coord(
                                    self,
                                    state,
                                    coord,
                                    Transaction {
                                        resource_man,
                                        tick_count,
                                        item_stack: ItemStack {
                                            item,
                                            amount: extracting,
                                        },
                                        source_type: tile_type.clone(),
                                        direction,
                                        source: myself,
                                    },
                                );

                                state.tick_pause = true;
                            }
                        }
                    }
                    Transfer(id) => {
                        if id == &resource_man.registry.tile_ids.master_node {
                            if let Some(target) =
                                state.data.get("target").and_then(Data::as_coord).cloned()
                            {
                                send_to_tile_coord(
                                    self,
                                    state,
                                    state.coord + target,
                                    ExtractRequest {
                                        resource_man,
                                        tick_count,
                                        coord,
                                        direction,
                                    },
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            AdjacentState { fulfilled } => {
                state.adjacent_fulfilled = fulfilled;
            }
        }

        Ok(())
    }
}

fn send_to_tile_coord(
    myself: &TileEntity,
    state: &mut TileEntityState,
    coord: TileCoord,
    message: TileEntityMsg,
) {
    match myself
        .game
        .send_message(GameMsg::ForwardMsgToTile(coord, message))
    {
        Ok(_) => {}
        Err(_) => {
            state.data.remove("target");
        }
    }
}

fn random() -> u32 {
    thread_rng().next_u32()
}
