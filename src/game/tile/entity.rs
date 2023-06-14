use std::sync::Arc;

use egui::NumExt;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};

use crate::game::item::{ItemAmount, ItemStack};
use crate::game::state::GameMsg;
use crate::game::tile::coord::TileCoord;
use crate::game::tile::entity::TileEntityMsg::*;
use crate::game::tile::entity::TransactionError::*;
use crate::game::tile::inventory::{Inventory, InventoryRaw};
use crate::game::tile::ticking::TickUnit;
use crate::resource::item::id_match;
use crate::resource::ResourceManager;
use crate::util::id::{Id, IdRaw, Interner};

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

/// Represents the data a tile entity holds. This data is given to functions.
#[derive(Debug, Clone)]
pub enum Data {
    /// The tile entity's inventory.
    Inventory(Inventory),
    /// The coordinates of the tile.
    Coord(TileCoord),
    /// The tile's ID.
    VecCoord(Vec<TileCoord>),
    Id(Id),
    VecId(Vec<Id>),
    Amount(ItemAmount),
    Bool(bool),
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
        if let Self::Inventory(v) = self {
            return Some(v);
        }
        None
    }

    /// Gets a mutable reference to  the tile's coordinates, or None.
    pub fn as_coord_mut(&mut self) -> Option<&mut TileCoord> {
        if let Self::Coord(v) = self {
            return Some(v);
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
        if let Self::VecCoord(v) = self {
            return Some(v);
        }
        None
    }

    /// Gets a mutable reference to the tile's ID, or None.
    pub fn as_id_mut(&mut self) -> Option<&mut Id> {
        if let Self::Id(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_vec_id_mut(&mut self) -> Option<&mut Vec<Id>> {
        if let Self::VecId(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_amount_mut(&mut self) -> Option<&mut ItemAmount> {
        if let Self::Amount(v) = self {
            return Some(v);
        }
        None
    }

    /// Gets an immutable reference to  the tile's Inventory, or None.
    pub fn as_inventory(&self) -> Option<&Inventory> {
        if let Self::Inventory(v) = self {
            return Some(v);
        }
        None
    }

    /// Gets an immutable reference to  the tile's coordinates, or None.
    pub fn as_coord(&self) -> Option<&TileCoord> {
        if let Self::Coord(v) = self {
            return Some(v);
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
        if let Self::VecCoord(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_id(&self) -> Option<&Id> {
        if let Self::Id(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_vec_id(&self) -> Option<&Vec<Id>> {
        if let Self::VecId(v) = self {
            return Some(v);
        }
        None
    }

    pub fn as_amount(&self) -> Option<&ItemAmount> {
        if let Self::Amount(v) = self {
            return Some(v);
        }
        None
    }
}

pub type DataMap = std::collections::BTreeMap<Id, Data>;

#[derive(Debug, Serialize, Deserialize)]
pub enum DataRaw {
    Inventory(InventoryRaw),
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    Id(IdRaw),
    VecId(Vec<IdRaw>),
    Amount(ItemAmount),
    Bool(bool),
}

pub type DataMapRaw = std::collections::HashMap<String, DataRaw>;

pub fn data_to_raw(data: DataMap, interner: &Interner) -> DataMapRaw {
    data.into_iter()
        .flat_map(|(key, value)| {
            let value = match value {
                Data::Inventory(inventory) => {
                    DataRaw::Inventory(InventoryRaw::from_inventory(inventory, interner))
                }
                Data::Coord(coord) => DataRaw::Coord(coord),
                Data::VecCoord(vec) => DataRaw::VecCoord(vec),
                Data::Id(id) => DataRaw::Id(IdRaw::parse(interner.resolve(id).unwrap())),
                Data::VecId(set) => DataRaw::VecId(
                    set.into_iter()
                        .map(|id| IdRaw::parse(interner.resolve(id).unwrap()))
                        .collect(),
                ),
                Data::Amount(amount) => DataRaw::Amount(amount),
                Data::Bool(v) => DataRaw::Bool(v),
            };

            interner.resolve(key).map(|key| (key.to_string(), value))
        })
        .collect()
}

pub fn intern_data_from_raw(data: DataMapRaw, interner: &mut Interner) -> DataMap {
    data.into_iter()
        .map(|(key, value)| {
            let value = match value {
                DataRaw::Inventory(inventory) => {
                    Data::Inventory(inventory.intern_to_inventory(interner))
                }
                DataRaw::Coord(coord) => Data::Coord(coord),
                DataRaw::VecCoord(vec) => Data::VecCoord(vec),
                DataRaw::Id(id) => Data::Id(interner.get_or_intern(id.to_string())),
                DataRaw::VecId(set) => Data::VecId(
                    set.into_iter()
                        .map(|id| interner.get_or_intern(id.to_string()))
                        .collect(),
                ),
                DataRaw::Amount(amount) => Data::Amount(amount),
                DataRaw::Bool(v) => Data::Bool(v),
            };

            (IdRaw::parse(&key).to_id(interner), value)
        })
        .collect()
}

pub fn data_from_raw(data: DataMapRaw, interner: &Interner) -> DataMap {
    data.into_iter()
        .flat_map(|(key, value)| {
            let value = match value {
                DataRaw::Inventory(inventory) => Data::Inventory(inventory.to_inventory(interner)),
                DataRaw::Coord(coord) => Data::Coord(coord),
                DataRaw::VecCoord(vec) => Data::VecCoord(vec),
                DataRaw::Id(id) => Data::Id(interner.get(id.to_string()).unwrap()),
                DataRaw::VecId(set) => Data::VecId(
                    set.into_iter()
                        .flat_map(|id| interner.get(id.to_string()))
                        .collect(),
                ),
                DataRaw::Amount(amount) => Data::Amount(amount),
                DataRaw::Bool(v) => Data::Bool(v),
            };

            interner
                .get(IdRaw::parse(&key).to_string())
                .map(|key| (key, value))
        })
        .collect()
}

/// Represents a tile entity. A tile entity is the actor that allows the tile to take, process, and output resources.
#[derive(Debug, Clone)]
pub struct TileEntityState {
    /// The ID of the tile entity.
    pub id: Id,
    //TODO can this be moved
    /// The coordinates of the tile entity.
    pub coord: TileCoord,
    /// The tile state of the tile entity.
    pub tile_modifier: TileModifier,
    /// The data map stored by the tile.
    pub data: DataMap,

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

            adjacent_fulfilled: false,
        }
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
        source_type: Option<Id>,
        source_id: Id,
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
    SetData(Id, Data),
    RemoveData(Id),
    GetData(RpcReplyPort<DataMap>),
    GetDataValue(Id, RpcReplyPort<Option<Data>>),
    GetDataValueAndCoord(Id, RpcReplyPort<(TileCoord, Option<Data>)>),
}

struct MachineTickResult {
    pub target: TileCoord,
    pub direction: TileCoord,
    pub item_stack: ItemStack,
}

pub struct TileEntity {
    pub(crate) game: ActorRef<GameMsg>,
}

impl TileEntity {
    fn machine_tick(
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
            let coord = state.coord + target;

            if let Some(output) = &script.instructions.output {
                if let Some(inputs) = &script.instructions.inputs {
                    if let Some(buffer) = state
                        .data
                        .get_mut(&resource_man.registry.data_ids.buffer)
                        .and_then(Data::as_inventory_mut)
                    {
                        for input in inputs {
                            let stored = buffer.get(input.item.id);

                            if stored < input.amount {
                                return None;
                            }
                        }

                        return Some(MachineTickResult {
                            target: coord,
                            direction: -target,
                            item_stack: *output,
                        });
                    }
                } else {
                    return Some(MachineTickResult {
                        target: coord,
                        direction: -target,
                        item_stack: *output,
                    });
                }
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
            return Err(NoScript);
        }

        if let Some(inputs) = script
            .and_then(Data::as_id)
            .and_then(|id| resource_man.registry.script(*id))
            .and_then(|script| script.instructions.inputs.as_ref())
        {
            if let Data::Inventory(buffer) = state
                .data
                .entry(resource_man.registry.data_ids.buffer)
                .or_insert_with(Data::inventory)
            {
                let matched = inputs
                    .iter()
                    .find(|v| id_match(&resource_man.registry, item_stack.item.id, v.item.id));

                if matched.is_none() {
                    return Err(NotSuitable);
                }

                let matched = matched.unwrap();

                let amount = buffer.get_mut(item_stack.item.id);
                if *amount >= matched.amount {
                    return Err(Full);
                }

                *amount += item_stack.amount;
                if *amount > matched.amount {
                    *amount = matched.amount;
                }

                return Ok(*matched);
            }
        }

        Err(NotSuitable)
    }
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
                let tile_type = resource_man.registry.tile(state.id).unwrap().tile_type;

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

                    if let Some(MachineTickResult {
                        target,
                        direction,
                        item_stack,
                    }) = TileEntity::machine_tick(state, &resource_man)
                    {
                        send_to_tile_coord(
                            self,
                            state,
                            target,
                            Transaction {
                                resource_man: resource_man.clone(),
                                tick_count,
                                item_stack,
                                source_type: tile_type,
                                source_id: state.id,
                                direction,
                                source: myself,
                            },
                            &resource_man,
                        );
                    }
                }

                if state.id == resource_man.registry.tile_ids.node {
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
                        send_to_tile_coord(
                            self,
                            state,
                            state.coord + link,
                            ExtractRequest {
                                resource_man: resource_man.clone(),
                                tick_count,
                                coord: state.coord + target,
                                direction: -target,
                            },
                            &resource_man,
                        );
                    }
                }
            }
            Transaction {
                resource_man,
                tick_count,
                item_stack,
                source_type: _,
                source_id: _,
                direction,
                source,
            } => {
                if state.id == resource_man.registry.tile_ids.void {
                    source
                        .send_message(TransactionResult {
                            resource_man: resource_man.clone(),
                            result: Ok(item_stack),
                        })
                        .unwrap()
                }

                let tile = resource_man.registry.tile(state.id).unwrap();

                if tile.tile_type == Some(resource_man.registry.tile_ids.machine) {
                    let result = TileEntity::machine_transaction(state, &resource_man, item_stack);

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
                        if item_stack.item == item {
                            let buffer = state
                                .data
                                .entry(resource_man.registry.data_ids.buffer)
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
                                        resource_man: resource_man.clone(),
                                        result: Err(Full),
                                    })
                                    .unwrap();

                                return Ok(());
                            }

                            let inserting = item_stack.amount.at_most(amount - *stored);
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
                            result: Err(NotSuitable),
                        })
                        .unwrap();
                }

                if state.id == resource_man.registry.tile_ids.merger {
                    if let Some(target) = state
                        .data
                        .get(&resource_man.registry.data_ids.target)
                        .and_then(Data::as_coord)
                        .cloned()
                    {
                        let target_coord = state.coord + target;

                        send_to_tile_coord(
                            self,
                            state,
                            target_coord,
                            Transaction {
                                resource_man: resource_man.clone(),
                                tick_count,
                                item_stack,
                                source_type: Some(resource_man.registry.tile_ids.transfer),
                                source_id: state.id,
                                direction: -target,
                                source: source.clone(),
                            },
                            &resource_man,
                        )
                    }
                }

                if state.id == resource_man.registry.tile_ids.splitter {
                    if let Some((a, b, c)) = match state.tile_modifier {
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
                        let (first, second) = if direction == a {
                            (b, c)
                        } else if direction == b {
                            (a, c)
                        } else {
                            (a, b)
                        };

                        let target = if random() % 2 == 0 { first } else { second };

                        let target_coord = state.coord + target;

                        send_to_tile_coord(
                            self,
                            state,
                            target_coord,
                            Transaction {
                                resource_man: resource_man.clone(),
                                tick_count,
                                item_stack,
                                source_type: Some(resource_man.registry.tile_ids.transfer),
                                source_id: state.id,
                                direction: -target,
                                source: source,
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
                    let tile_type = resource_man.registry.tile(state.id).unwrap().tile_type;

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

                    if tile_type == Some(resource_man.registry.tile_ids.storage) {
                        if let Some(buffer) = state
                            .data
                            .get_mut(&resource_man.registry.data_ids.buffer)
                            .and_then(Data::as_inventory_mut)
                        {
                            buffer.take(transferred.item.id, transferred.amount);
                        }
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
                reply.send(state.data.get(&key).cloned()).unwrap();
            }
            GetDataValueAndCoord(key, reply) => {
                reply
                    .send((state.coord, state.data.get(&key).cloned()))
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
                let tile_type = resource_man.registry.tile(state.id).unwrap().tile_type;

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
                        let stored = inventory.get(item.id);
                        let extracting = stored.min(amount);

                        if extracting > 0 {
                            send_to_tile_coord(
                                self,
                                state,
                                coord,
                                Transaction {
                                    resource_man: resource_man.clone(),
                                    tick_count,
                                    item_stack: ItemStack {
                                        item,
                                        amount: extracting,
                                    },
                                    source_type: tile_type,
                                    source_id: state.id,
                                    direction,
                                    source: myself,
                                },
                                &resource_man,
                            );
                        }
                    }
                }

                if state.id == resource_man.registry.tile_ids.master_node {
                    if let Some(target) = state
                        .data
                        .get(&resource_man.registry.data_ids.target)
                        .and_then(Data::as_coord)
                        .cloned()
                    {
                        send_to_tile_coord(
                            self,
                            state,
                            state.coord + target,
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

fn send_to_tile_coord(
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
