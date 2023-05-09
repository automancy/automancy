use std::sync::Arc;

use egui::NumExt;

use rand::{thread_rng, RngCore};
use riker::actor::{Context, Sender};
use riker::actors::{Actor, ActorFactoryArgs, BasicActorRef};
use rune::{Any, ContextError, Module, Vm};
use serde::{Deserialize, Serialize};

use crate::game::inventory::{Inventory, InventoryRaw};
use crate::game::item::{ItemAmount, ItemStack};
use crate::game::tile::coord::TileCoord;
use crate::game::tile::entity::TileEntityMsg::*;
use crate::game::tile::entity::TransactionError::*;
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
pub struct TileEntity {
    /// The ID of the tile entity.
    #[rune(get, copy)]
    id: Id,
    /// The coordinates of the tile entity.
    #[rune(get, copy)]
    coord: TileCoord,
    /// The tile state of the tile entity.
    #[rune(get, copy)]
    tile_state: TileState,
    /// The data map stored by the tile.
    #[rune(get, set)] // TODO use methods
    data: DataMap,
    /// The actor system.
    game: BasicActorRef,

    tick_pause: bool,
    adjacent_fulfilled: bool,
}

impl TileEntity {
    /// Creates a new tile entity.
    fn new(game: BasicActorRef, id: Id, coord: TileCoord, tile_state: TileState) -> Self {
        Self {
            id,
            coord,
            tile_state,
            data: DataMap::default(),

            game,
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

pub type TileState = i32;

#[derive(Debug, Clone)]
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
    GetData,
    GetDataValue(&'static str),
    GetDataValueAndCoord(&'static str),
}

impl Actor for TileEntity {
    type Msg = TileEntityMsg;

    fn post_stop(&mut self) {
        self.data.clear();
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        let myself = Some(ctx.myself().into());

        match msg {
            Tick {
                resource_man,
                tick_count,
            } => {
                self.tick_pause = false;

                let tile_type = &resource_man.registry.get_tile(&self.id).unwrap().tile_type;

                match tile_type {
                    Machine(_) => {
                        if tick_count % 10 == 0 {
                            if let Some(script) = self.data.get("script").and_then(Data::as_id) {
                                if let Some(script) = resource_man.registry.get_script(script) {
                                    self.game
                                        .try_tell(
                                            GameMsg::CheckAdjacent {
                                                script,
                                                coord: self.coord,
                                            },
                                            myself.clone(),
                                        )
                                        .unwrap();
                                }
                            }
                        }

                        if !self.adjacent_fulfilled {
                            return;
                        }

                        let function =
                            resource_man.functions[&resource_man.registry.tile_ids.machine].clone();
                        let mut vm = Vm::new(function.context, function.unit);

                        let output = vm
                            .call(
                                ["handle_tick"],
                                (&mut *self, &resource_man.registry, tile_type, tick_count),
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

                            self.send_tile_msg(
                                myself,
                                target_coord,
                                Transaction {
                                    resource_man: resource_man.clone(),
                                    tick_count,
                                    item_stack,
                                    source_type: tile_type.clone(),
                                    direction,
                                },
                            );
                        }
                    }
                    Transfer(id) => {
                        if id == &resource_man.registry.tile_ids.node {
                            if let Some((target, link)) = self
                                .data
                                .get("target")
                                .and_then(Data::as_coord)
                                .cloned()
                                .zip(self.data.get("link").and_then(Data::as_coord).cloned())
                            {
                                self.send_tile_msg(
                                    myself,
                                    self.coord + link,
                                    ExtractRequest {
                                        resource_man,
                                        tick_count,
                                        coord: self.coord + target,
                                        direction: -target,
                                    },
                                )
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
            } => {
                if let Some(sender) = sender {
                    let tile_type = resource_man.registry.get_tile(&self.id).unwrap().tile_type;

                    match &tile_type {
                        Machine(_) => {
                            let function = resource_man.functions
                                [&resource_man.registry.tile_ids.machine]
                                .clone();
                            let mut vm = Vm::new(function.context, function.unit);

                            let output = vm
                                .call(
                                    ["handle_transaction"],
                                    (
                                        &mut *self,
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

                                sender
                                    .try_tell(
                                        TransactionResult {
                                            result,
                                            resource_man,
                                        },
                                        myself,
                                    )
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
                                            &mut *self,
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

                                    self.send_tile_msg(
                                        Some(sender),
                                        target_coord,
                                        Transaction {
                                            resource_man,
                                            tick_count,
                                            item_stack,
                                            source_type: tile_type,
                                            direction: -target,
                                        },
                                    );
                                }
                            }
                        }
                        Void => {
                            sender
                                .try_tell(
                                    TransactionResult {
                                        resource_man,
                                        result: Ok(item_stack),
                                    },
                                    myself,
                                )
                                .unwrap();
                        }
                        Storage(_) => {
                            if let Some((item, amount)) = self
                                .data
                                .get("storage")
                                .and_then(Data::as_id)
                                .and_then(|id| resource_man.registry.get_item(id))
                                .zip(self.data.get("amount").and_then(Data::as_amount).cloned())
                            {
                                if item_stack.item == item {
                                    let buffer = self
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
                                        sender
                                            .try_tell(
                                                TransactionResult {
                                                    resource_man,
                                                    result: Err(Full),
                                                },
                                                myself,
                                            )
                                            .unwrap();
                                        return;
                                    }

                                    let inserting = item_stack.amount.at_most(amount - *stored);
                                    *stored += inserting;

                                    sender
                                        .try_tell(
                                            TransactionResult {
                                                resource_man,
                                                result: Ok(ItemStack {
                                                    item,
                                                    amount: inserting,
                                                }),
                                            },
                                            myself,
                                        )
                                        .unwrap();
                                    return;
                                }
                            }
                            sender
                                .try_tell(
                                    TransactionResult {
                                        resource_man,
                                        result: Err(NotSuitable),
                                    },
                                    myself,
                                )
                                .unwrap();
                        }
                        _ => {
                            sender
                                .try_tell(
                                    TransactionResult {
                                        resource_man,
                                        result: Err(NotSuitable),
                                    },
                                    myself,
                                )
                                .unwrap();
                        }
                    }
                }
            }
            TransactionResult {
                resource_man,
                result,
            } => {
                if let Ok(transferred) = result {
                    let tile_type = &resource_man.registry.get_tile(&self.id).unwrap().tile_type;

                    match tile_type {
                        Machine(_) => {
                            if let Some(inputs) = self
                                .data
                                .get("script")
                                .and_then(Data::as_id)
                                .and_then(|script| resource_man.registry.get_script(script))
                                .and_then(|script| script.instructions.inputs)
                            {
                                let buffer = self
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
                            if let Some(buffer) =
                                self.data.get_mut("buffer").and_then(Data::as_inventory_mut)
                            {
                                buffer.take(transferred.item.id, transferred.amount);
                            }
                        }
                        _ => {}
                    }
                }
            }
            SetData(key, value) => {
                self.data.insert(key, value);
            }
            GetData => {
                if let Some(sender) = sender {
                    sender.try_tell(self.data.clone(), myself).unwrap()
                }
            }
            GetDataValue(key) => {
                if let Some(sender) = sender {
                    sender
                        .try_tell(self.data.get(key).cloned(), myself)
                        .unwrap()
                }
            }
            GetDataValueAndCoord(key) => {
                if let Some(sender) = sender {
                    sender
                        .try_tell((self.coord, self.data.get(key).cloned()), myself)
                        .unwrap()
                }
            }
            RemoveData(key) => {
                self.data.remove(key);
            }
            ExtractRequest {
                resource_man,
                tick_count,
                coord,
                direction,
            } => {
                if self.tick_pause {
                    return;
                }

                let tile_type = &resource_man.registry.get_tile(&self.id).unwrap().tile_type;

                match tile_type {
                    Storage(_) => {
                        if let Some(((item, amount), inventory)) = self
                            .data
                            .get("storage")
                            .and_then(Data::as_id)
                            .and_then(|id| resource_man.registry.get_item(id))
                            .zip(self.data.get("amount").and_then(Data::as_amount).cloned())
                            .zip(self.data.get_mut("buffer").and_then(Data::as_inventory_mut))
                        {
                            let stored = inventory.get(item.id);
                            let extracting = stored.min(amount);

                            if extracting > 0 {
                                self.send_tile_msg(
                                    myself,
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
                                    },
                                );
                                self.tick_pause = true;
                            }
                        }
                    }
                    Transfer(id) => {
                        if id == &resource_man.registry.tile_ids.master_node {
                            if let Some(target) =
                                self.data.get("target").and_then(Data::as_coord).cloned()
                            {
                                self.send_tile_msg(
                                    sender,
                                    self.coord + target,
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
                self.adjacent_fulfilled = fulfilled;
            }
        }
    }
}

impl TileEntity {
    fn send_tile_msg(
        &mut self,
        sender: Option<BasicActorRef>,
        coord: TileCoord,
        msg: TileEntityMsg,
    ) {
        match self
            .game
            .try_tell(GameMsg::ForwardMsgToTile(coord, msg), sender)
        {
            Ok(_) => {}
            Err(_) => {
                self.data.remove("target");
            }
        }
    }
}

impl ActorFactoryArgs<(BasicActorRef, Id, TileCoord, TileState)> for TileEntity {
    fn create_args(args: (BasicActorRef, Id, TileCoord, TileState)) -> Self {
        Self::new(args.0, args.1, args.2, args.3)
    }
}

fn random() -> u32 {
    thread_rng().next_u32()
}
