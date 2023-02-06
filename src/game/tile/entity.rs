use std::sync::Arc;

use egui::NumExt;

use riker::actor::{Context, Sender};
use riker::actors::{Actor, ActorFactoryArgs, BasicActorRef};
use rune::{FromValue, Vm};

use crate::game::inventory::Inventory;
use crate::game::item::ItemStack;
use crate::game::tile::coord::TileCoord;
use crate::game::tile::entity::TileEntityMsg::*;
use crate::game::{GameMsg, TickUnit};
use crate::resource::item::id_eq_or_of_tag;
use crate::resource::script::Script;
use crate::resource::tile::TileType;
use crate::resource::tile::TileType::*;
use crate::resource::ResourceManager;
use crate::util::id::Id;

#[derive(Debug, Copy, Clone)]
pub enum TransactionError {
    NoScript,
    NotSuitable,
    Full,
}

#[derive(Debug, Clone)]
pub struct TileEntity {
    id: Id,

    data: Inventory,
    coord: TileCoord,
    script: Option<Id>,

    game: BasicActorRef,
    target_direction: Option<TileCoord>,

    tile_state: StateUnit,
}

pub type StateUnit = i32;

#[derive(Debug, Clone)]
pub enum TileEntityMsg {
    Tick {
        resource_man: Arc<ResourceManager>,
        tick_count: TickUnit,
    },
    Transaction {
        resource_man: Arc<ResourceManager>,
        tick_count: TickUnit,
        item: ItemStack,
        source_type: TileType,
        direction: TileCoord,
    },
    TransactionResult(Result<(), TransactionError>, Arc<ResourceManager>),

    SetTarget(Option<TileCoord>),
    GetTarget,

    SetScript(Id, Arc<ResourceManager>),
    GetScript,

    SetData(Inventory),
    GetData,
}

impl Actor for TileEntity {
    type Msg = TileEntityMsg;

    fn post_stop(&mut self) {
        self.script = None;
        self.target_direction = None;
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        let myself = Some(ctx.myself().into());

        match msg {
            Tick {
                resource_man,
                tick_count,
            } => {
                let tile_type = resource_man.tiles[&self.id].tile_type.clone();

                if let Machine(_) = tile_type {
                    self.machine_tell(myself, resource_man, tile_type, tick_count);
                }
            }
            Transaction {
                resource_man,
                tick_count,
                item: item_stack,
                source_type,
                direction,
            } => {
                if let Some(sender) = sender {
                    let tile_type = resource_man.tiles[&self.id].tile_type.clone();

                    match &tile_type {
                        Machine(_) => {
                            self.machine_result(myself, sender, resource_man, item_stack);
                        }
                        Transfer(id) => {
                            if let Transfer(_) = source_type {
                                return;
                            }

                            let function = resource_man.functions[id].clone();
                            let mut vm = Vm::new(function.context, function.unit);

                            let output = vm
                                .call(
                                    ["handle_transfer"],
                                    (self.tile_state, tick_count, self.coord, direction),
                                )
                                .unwrap();

                            if let Some((target_coord, target)) =
                                <Option<(TileCoord, TileCoord)>>::from_value(output).unwrap()
                            {
                                self.send_tile_msg(
                                    Some(sender),
                                    target_coord,
                                    Transaction {
                                        resource_man,
                                        tick_count,
                                        item: item_stack,
                                        source_type: tile_type.clone(),
                                        direction: target,
                                    },
                                );
                            }
                        }
                        Void => {
                            sender
                                .try_tell(TransactionResult(Ok(()), resource_man.clone()), myself)
                                .unwrap();
                        }
                        Storage(_) => {
                            if let Some(script) = self.get_script(resource_man.clone()) {
                                if let Some(output) = script.instructions.output {
                                    if id_eq_or_of_tag(
                                        &resource_man,
                                        item_stack.item.id,
                                        output.item.id,
                                    ) {
                                        let amount =
                                            self.data.0.entry(item_stack.item.id).or_insert(0);

                                        if *amount == output.amount {
                                            sender
                                                .try_tell(
                                                    TransactionResult(
                                                        Err(TransactionError::Full),
                                                        resource_man.clone(),
                                                    ),
                                                    myself,
                                                )
                                                .unwrap();
                                            return;
                                        }

                                        *amount += item_stack.amount;
                                        *amount = amount.at_most(output.amount);

                                        sender
                                            .try_tell(
                                                TransactionResult(Ok(()), resource_man.clone()),
                                                myself,
                                            )
                                            .unwrap();
                                        return;
                                    }
                                }
                            }
                            sender
                                .try_tell(
                                    TransactionResult(
                                        Err(TransactionError::NotSuitable),
                                        resource_man.clone(),
                                    ),
                                    myself,
                                )
                                .unwrap();
                        }
                        _ => {
                            sender
                                .try_tell(
                                    TransactionResult(
                                        Err(TransactionError::NotSuitable),
                                        resource_man.clone(),
                                    ),
                                    myself,
                                )
                                .unwrap();
                        }
                    }
                }
            }
            TransactionResult(result, resource_man) => match result {
                Ok(_) => {
                    if let Some(input) = self
                        .get_script(resource_man)
                        .and_then(|script| script.instructions.input)
                    {
                        let stored = *self.data.0.get(&input.item.id).unwrap_or(&0);
                        if stored < input.amount {
                            log::error!("in transaction result: tile does not have enough input for the supposed output!");
                            return;
                        }

                        self.data.0.insert(input.item.id, stored - input.amount);
                    }
                }
                _ => {}
            },
            SetTarget(target_direction) => {
                self.target_direction = target_direction;
            }
            GetTarget => {
                sender.inspect(|v| v.try_tell(self.target_direction, myself).unwrap());
            }
            SetScript(id, resource_man) => {
                self.script = Some(id);

                if let Some(input) = resource_man
                    .scripts
                    .get(&id)
                    .and_then(|script| script.instructions.input)
                {
                    self.data.0 = self
                        .data
                        .0
                        .iter()
                        .map(|(id, amount)| {
                            if id_eq_or_of_tag(&resource_man, *id, input.item.id) {
                                Some((*id, amount.at_most(input.amount)))
                            } else {
                                None
                            }
                        })
                        .flatten()
                        .collect();
                } else {
                    self.data.0.clear();
                }
            }
            GetScript => {
                sender.inspect(|v| v.try_tell(self.script, myself).unwrap());
            }
            SetData(data) => {
                self.data = data;
            }
            GetData => {
                sender.inspect(|v| v.try_tell(self.data.clone(), myself).unwrap());
            }
        }
    }
}

impl ActorFactoryArgs<(BasicActorRef, Id, TileCoord, Inventory, StateUnit)> for TileEntity {
    fn create_args(args: (BasicActorRef, Id, TileCoord, Inventory, StateUnit)) -> Self {
        Self::new(args.0, args.1, args.2, args.3, args.4)
    }
}

impl TileEntity {
    fn machine_tell(
        &mut self,
        myself: Option<BasicActorRef>,
        resource_man: Arc<ResourceManager>,
        tile_type: TileType,
        tick_count: TickUnit,
    ) {
        if let Some(direction) = self.target_direction {
            let coord = self.coord + direction;

            if let Some(script) = self.get_script(resource_man.clone()) {
                let instructions = &script.instructions;
                let output = instructions.output;

                if let Some(output) = output {
                    if let Some(input) = instructions.input {
                        let stored = *self.data.0.get(&input.item.id).unwrap_or(&0);
                        if stored >= input.amount {
                            self.send_tile_msg(
                                myself,
                                coord,
                                Transaction {
                                    resource_man,
                                    tick_count,
                                    item: output,
                                    source_type: tile_type,
                                    direction,
                                },
                            );
                        }
                    } else {
                        self.send_tile_msg(
                            myself,
                            coord,
                            Transaction {
                                resource_man,
                                tick_count,
                                item: output,
                                source_type: tile_type,
                                direction,
                            },
                        );
                    }
                }
            }
        }
    }

    fn machine_result(
        &mut self,
        myself: Option<BasicActorRef>,
        sender: BasicActorRef,
        resource_man: Arc<ResourceManager>,
        item: ItemStack,
    ) {
        if let Some(script) = self.get_script(resource_man.clone()) {
            if let Some(input) = script.instructions.input {
                if !id_eq_or_of_tag(&resource_man, item.item.id, input.item.id) {
                    sender
                        .try_tell(
                            TransactionResult(
                                Err(TransactionError::NotSuitable),
                                resource_man.clone(),
                            ),
                            myself,
                        )
                        .unwrap();
                    return;
                }

                let amount = self.data.0.entry(item.item.id).or_insert(0);
                if *amount == input.amount {
                    sender
                        .try_tell(
                            TransactionResult(Err(TransactionError::Full), resource_man.clone()),
                            myself,
                        )
                        .unwrap();
                    return;
                }

                *amount += item.amount;
                *amount = amount.at_most(input.amount);

                sender
                    .try_tell(TransactionResult(Ok(()), resource_man.clone()), myself)
                    .unwrap();
            } else {
                sender
                    .try_tell(
                        TransactionResult(Err(TransactionError::NotSuitable), resource_man.clone()),
                        myself,
                    )
                    .unwrap();
            }
        } else {
            sender
                .try_tell(
                    TransactionResult(Err(TransactionError::NoScript), resource_man.clone()),
                    myself,
                )
                .unwrap();
        }
    }

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
                self.target_direction = None;
            }
        }
    }

    fn get_script(&self, resource_man: Arc<ResourceManager>) -> Option<Script> {
        self.script
            .as_ref()
            .and_then(|v| resource_man.scripts.get(v))
            .cloned()
    }

    fn new(
        game: BasicActorRef,
        id: Id,
        coord: TileCoord,
        data: Inventory,
        tile_state: StateUnit,
    ) -> Self {
        Self {
            id,
            coord,
            data,
            script: None,

            game,
            target_direction: None,

            tile_state,
        }
    }
}
