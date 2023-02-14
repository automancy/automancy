use egui::NumExt;
use std::collections::HashMap;
use std::sync::Arc;

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

#[derive(Debug, Copy, Clone, Any)]
pub enum TransactionError {
    #[rune(constructor)]
    NoScript,
    #[rune(constructor)]
    NotSuitable,
    #[rune(constructor)]
    Full,
}

#[derive(Debug, Clone, Any)]
pub enum Data {
    #[rune(constructor)]
    Inventory(#[rune(get)] Inventory),

    #[rune(constructor)]
    Coord(#[rune(get, copy)] TileCoord),

    #[rune(constructor)]
    VecCoord(#[rune(get)] Vec<TileCoord>),

    #[rune(constructor)]
    Id(#[rune(get, copy)] Id),

    #[rune(constructor)]
    Amount(#[rune(get)] ItemAmount),
}

impl Data {
    pub fn inventory() -> Self {
        Self::Inventory(Default::default())
    }
    pub fn vec_coord() -> Self {
        Self::VecCoord(Default::default())
    }

    pub fn as_inventory_mut(&mut self) -> Option<&mut Inventory> {
        if let Self::Inventory(inventory) = self {
            return Some(inventory);
        }
        None
    }

    pub fn as_coord_mut(&mut self) -> Option<&mut TileCoord> {
        if let Self::Coord(coord) = self {
            return Some(coord);
        }
        None
    }

    pub fn as_vec_coord_mut(&mut self) -> Option<&mut Vec<TileCoord>> {
        if let Self::VecCoord(vec) = self {
            return Some(vec);
        }
        None
    }

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

    pub fn as_inventory(&self) -> Option<&Inventory> {
        if let Self::Inventory(inventory) = self {
            return Some(inventory);
        }
        None
    }

    pub fn as_coord(&self) -> Option<&TileCoord> {
        if let Self::Coord(coord) = self {
            return Some(coord);
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

pub type DataMap = HashMap<String, Data>;

#[derive(Debug, Serialize, Deserialize)]
pub enum DataRaw {
    Inventory(InventoryRaw),
    Coord(TileCoord),
    VecCoord(Vec<TileCoord>),
    Id(IdRaw),
    Amount(ItemAmount),
}

pub type DataMapRaw = HashMap<String, DataRaw>;

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
            };

            (key, value)
        })
        .collect()
}

#[derive(Debug, Clone, Any)]
pub struct TileEntity {
    #[rune(get, copy)]
    id: Id,
    #[rune(get, copy)]
    coord: TileCoord,
    #[rune(get, copy)]
    tile_state: TileState,

    #[rune(get, set)]
    data: DataMap,

    game: BasicActorRef,
}

impl TileEntity {
    fn new(game: BasicActorRef, id: Id, coord: TileCoord, tile_state: TileState) -> Self {
        Self {
            id,
            coord,
            tile_state,
            data: DataMap::default(),

            game,
        }
    }

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
        result: Result<(), TransactionError>,
    },

    SetData(String, Data),
    RemoveData(String),
    GetData,
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
                let tile_type = &resource_man.registry.get_tile(self.id).unwrap().tile_type;

                if let Machine(_) = tile_type {
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
            }
            Transaction {
                resource_man,
                tick_count,
                item_stack,
                source_type,
                direction,
            } => {
                if let Some(sender) = sender {
                    let tile_type = &resource_man.registry.get_tile(self.id).unwrap().tile_type;

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
                                    ),
                                )
                                .unwrap();

                            if let Ok(output) = output.into_result() {
                                let result: Result<(), TransactionError> =
                                    output.take().unwrap().map(|_v| ()).map_err(|v| {
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
                            if let Transfer(_) = &source_type {
                                return;
                            }

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
                                            source_type,
                                            direction: target,
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
                                        result: Ok(()),
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
                                .and_then(|id| resource_man.registry.get_item(*id))
                                .zip(self.data.get("amount").and_then(Data::as_amount).cloned())
                            {
                                if item_stack.item == item {
                                    let buffer = self
                                        .data
                                        .entry("buffer".to_string())
                                        .or_insert_with(Data::inventory)
                                        .as_inventory_mut()
                                        .unwrap();
                                    let stored = buffer.0.entry(item.id).or_insert(0);

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

                                    *stored += item_stack.amount;
                                    *stored = stored.at_most(amount);

                                    sender
                                        .try_tell(
                                            TransactionResult {
                                                resource_man,
                                                result: Ok(()),
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
                if result.is_ok() {
                    if let Some(input) = self
                        .data
                        .get("script")
                        .and_then(Data::as_id)
                        .and_then(|script| resource_man.registry.get_script(*script))
                        .and_then(|script| script.instructions.input)
                    {
                        let stored = self
                            .data
                            .get_mut("buffer")
                            .and_then(Data::as_inventory_mut)
                            .unwrap()
                            .0
                            .entry(input.item.id)
                            .or_insert(0);

                        if *stored < input.amount {
                            log::error!("in transaction result: tile does not have enough input for the supposed output!");
                            *stored = 0;
                        } else {
                            *stored -= input.amount
                        }
                    }
                }
            }
            SetData(key, value) => {
                self.data.insert(key, value);
            }
            GetData => {
                sender.inspect(|v| v.try_tell(self.data.clone(), myself).unwrap());
            }
            RemoveData(key) => {
                self.data.remove(&key);
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
            .try_tell(GameMsg::SendMsgToTile(coord, msg), sender)
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
