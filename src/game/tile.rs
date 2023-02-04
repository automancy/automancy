use std::sync::Arc;

use egui::NumExt;

use riker::actor::{Context, Sender};
use riker::actors::{Actor, ActorFactoryArgs, BasicActorRef};
use rune::{FromValue, Vm};

use crate::game::data::{Data, TileCoord};
use crate::game::item::Item;
use crate::game::tile::TileEntityMsg::*;
use crate::game::{GameMsg, TickUnit};
use crate::resource::script::Script;
use crate::resource::tile::TileType;
use crate::resource::tile::TileType::*;
use crate::resource::ResourceManager;
use crate::util::id::Id;

pub type StateUnit = i32;

#[derive(Debug, Clone)]
pub struct TileEntity {
    id: Id,
    coord: TileCoord,
    data: Data,
    script: Option<Id>,

    game: BasicActorRef,
    target_direction: Option<TileCoord>,

    tile_state: StateUnit,
}

#[derive(Debug, Copy, Clone)]
pub enum TransactionError {
    NotEnough,
    NoScript,
    NotSuitable,
    Full,
}

#[derive(Debug, Clone)]
pub enum TileEntityMsg {
    Tick {
        resource_man: Arc<ResourceManager>,
        tick_count: TickUnit,
    },
    Transaction {
        resource_man: Arc<ResourceManager>,
        tick_count: TickUnit,
        item: Item,
        source_type: TileType,
        direction: TileCoord,
    },
    TransactionResult(Result<(), TransactionError>),

    SetTarget(Option<TileCoord>),
    GetTarget,

    SetScript(Id),
    GetScript,

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
                item,
                source_type,
                direction,
            } => {
                if let Some(sender) = sender {
                    let tile_type = resource_man.tiles[&self.id].tile_type.clone();

                    match &tile_type {
                        Machine(_) => {
                            self.machine_result(myself, sender, resource_man, item);
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
                                        item,
                                        source_type: tile_type.clone(),
                                        direction: target,
                                    },
                                );
                            }
                        }
                        Void => {
                            sender.try_tell(TransactionResult(Ok(())), myself).unwrap();
                        }
                        _ => {
                            sender
                                .try_tell(
                                    TransactionResult(Err(TransactionError::NotSuitable)),
                                    myself,
                                )
                                .unwrap();
                        }
                    }
                }
            }
            TransactionResult(_) => {}
            SetTarget(target_direction) => {
                self.target_direction = target_direction;
            }
            GetTarget => {
                sender.inspect(|v| v.try_tell(self.target_direction, myself).unwrap());
            }
            SetScript(id) => {
                self.script = Some(id);
                self.data.0.clear();
            }
            GetScript => {
                sender.inspect(|v| v.try_tell(self.script, myself).unwrap());
            }
            GetData => {
                sender.inspect(|v| v.try_tell(self.data.clone(), myself).unwrap());
            }
        }
    }
}

impl ActorFactoryArgs<(BasicActorRef, Id, TileCoord, Data, StateUnit)> for TileEntity {
    fn create_args(args: (BasicActorRef, Id, TileCoord, Data, StateUnit)) -> Self {
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

                if let Some(input) = instructions.input {
                    let id = input.id;

                    // TODO send transaction result back to Game
                    let stored = *self.data.0.get(&id).unwrap_or(&0);
                    if stored >= input.amount {
                        self.data.0.insert(id, stored - input.amount);

                        if let Some(output) = output {
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
                } else if let Some(output) = output {
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

    fn machine_result(
        &mut self,
        myself: Option<BasicActorRef>,
        sender: BasicActorRef,
        resource_man: Arc<ResourceManager>,
        item: Item,
    ) {
        if let Some(script) = self.get_script(resource_man.clone()) {
            if let Some(input) = script.instructions.input {
                let id = item.id;
                let has = item.amount;
                let required = input.amount;

                if input.id != id {
                    sender
                        .try_tell(
                            TransactionResult(Err(TransactionError::NotSuitable)),
                            myself,
                        )
                        .unwrap();
                    return;
                }

                let amount = self.data.0.entry(id).or_insert(0);
                *amount += has;

                let limit = required * 2;

                if *amount > limit {
                    *amount = amount.at_most(limit);

                    sender
                        .try_tell(TransactionResult(Err(TransactionError::Full)), myself)
                        .unwrap();
                    return;
                }

                if has >= required {
                    sender.try_tell(TransactionResult(Ok(())), myself).unwrap();
                } else {
                    sender
                        .try_tell(TransactionResult(Err(TransactionError::NotEnough)), myself)
                        .unwrap();
                }
            } else {
                sender
                    .try_tell(
                        TransactionResult(Err(TransactionError::NotSuitable)),
                        myself,
                    )
                    .unwrap();
            }
        } else {
            sender
                .try_tell(TransactionResult(Err(TransactionError::NoScript)), myself)
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
        data: Data,
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
