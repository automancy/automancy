use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::Arc;

use egui::NumExt;
use hexagon_tiles::hex::{hex, Hex};
use hexagon_tiles::traits::HexDirection;

use riker::actor::{Context, Sender};
use riker::actors::{Actor, ActorFactoryArgs, BasicActorRef};
use rune::{Any, FromValue, Module, Vm};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::game::data::Data;
use crate::game::game::{GameMsg, TickUnit};
use crate::game::item::Item;
use crate::game::script::Script;
use crate::game::tile::TileEntityMsg::*;
use crate::util::id::Id;
use crate::util::resource::ResourceManager;
use crate::util::resource::TileType;
use crate::util::resource::TileType::*;

pub type StateUnit = i32;

#[derive(Debug, Clone)]
pub struct TileEntity {
    id: Id,
    coord: TileCoord,
    data: Data,
    script: Option<Id>,

    game: BasicActorRef,
    target_direction: Option<TileCoord>,

    interval_offset: TickUnit,

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
                let interval = 1 + self.interval_offset;

                if (tick_count % interval) != 0 {
                    return;
                }

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
                                self.game
                                    .try_tell(
                                        GameMsg::SendMsgToTile(
                                            target_coord,
                                            Transaction {
                                                resource_man,
                                                tick_count,
                                                item,
                                                source_type: tile_type.clone(),
                                                direction: target,
                                            },
                                        ),
                                        Some(sender),
                                    )
                                    .unwrap();
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
            TransactionResult(result) => match result {
                Ok(_) => {
                    self.interval_offset = 0;
                }
                Err(error) => match error {
                    TransactionError::NoScript
                    | TransactionError::NotSuitable
                    | TransactionError::Full => {
                        self.slow_down();
                    }
                    TransactionError::NotEnough => {
                        self.interval_offset = 0;
                    }
                },
            },
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

            if let Some(script) = self
                .script
                .as_ref()
                .and_then(|v| resource_man.scripts.get(v))
            {
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
        } else {
            self.slow_down();
        }
    }

    fn machine_result(
        &mut self,
        myself: Option<BasicActorRef>,
        sender: BasicActorRef,
        resource_man: Arc<ResourceManager>,
        item: Item,
    ) {
        if let Some(script) = self.get_script(resource_man) {
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

    fn slow_down(&mut self) {
        self.interval_offset += 2;
        self.interval_offset = self.interval_offset.at_most(9);
    }

    fn send_tile_msg(
        &mut self,
        myself: Option<BasicActorRef>,
        coord: TileCoord,
        msg: TileEntityMsg,
    ) {
        match self
            .game
            .try_tell(GameMsg::SendMsgToTile(coord, msg), myself)
        {
            Ok(_) => {}
            Err(_) => {
                self.target_direction = None;
            }
        }
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

            interval_offset: 0,

            tile_state,
        }
    }

    fn get_script(&self, resource_man: Arc<ResourceManager>) -> Option<Script> {
        self.script
            .as_ref()
            .and_then(|v| resource_man.scripts.get(v))
            .cloned()
    }
}

pub type TileHex = Hex<TileUnit>;

pub type TileUnit = i32;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Any)]
pub struct TileCoord(TileHex);

impl From<TileHex> for TileCoord {
    fn from(value: TileHex) -> Self {
        Self(value)
    }
}

impl From<TileCoord> for TileHex {
    fn from(value: TileCoord) -> Self {
        value.0
    }
}

impl TileCoord {
    pub fn install(module: &mut Module) -> Result<(), rune::ContextError> {
        module.ty::<Self>()?;
        module.inst_fn("neg", Self::neg)?;
        module.inst_fn("add", Self::add)?;
        module.inst_fn("sub", Self::sub)?;
        module.inst_fn("mul", Self::mul)?;
        module.inst_fn("div", Self::div)?;
        module.inst_fn("clone", Self::clone)?;
        module.function(["TOP_RIGHT"], || Self::TOP_RIGHT)?;
        module.function(["RIGHT"], || Self::RIGHT)?;
        module.function(["BOTTOM_RIGHT"], || Self::BOTTOM_RIGHT)?;
        module.function(["BOTTOM_LEFT"], || Self::BOTTOM_LEFT)?;
        module.function(["LEFT"], || Self::LEFT)?;
        module.function(["TOP_LEFT"], || Self::TOP_LEFT)?;

        Ok(())
    }

    pub const TOP_RIGHT: Self = Self(TileHex::NEIGHBORS[2]);
    pub const RIGHT: Self = Self(TileHex::NEIGHBORS[3]);
    pub const BOTTOM_RIGHT: Self = Self(TileHex::NEIGHBORS[4]);
    pub const BOTTOM_LEFT: Self = Self(TileHex::NEIGHBORS[5]);
    pub const LEFT: Self = Self(TileHex::NEIGHBORS[0]);
    pub const TOP_LEFT: Self = Self(TileHex::NEIGHBORS[1]);

    pub fn q(self) -> TileUnit {
        self.0.q()
    }

    pub fn r(self) -> TileUnit {
        self.0.r()
    }

    pub fn s(self) -> TileUnit {
        self.0.s()
    }
}

impl TileCoord {
    pub const ZERO: Self = Self(hex(0, 0, 0));

    pub fn new(q: TileUnit, r: TileUnit) -> Self {
        Self(Hex::new(q, r))
    }
}

impl Display for TileCoord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{}, {}]", self.0.q(), self.0.r()))
    }
}

impl Serialize for TileCoord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.0.q())?;
        tuple.serialize_element(&self.0.r())?;
        tuple.end()
    }
}

struct TileCoordVisitor;

impl<'de> Visitor<'de> for TileCoordVisitor {
    type Value = TileCoord;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let q: TileUnit = seq.next_element()?.unwrap();
        let r: TileUnit = seq.next_element()?.unwrap();

        Ok(TileCoord::new(q, r))
    }
}

impl<'de> Deserialize<'de> for TileCoord
where
    Self: Sized,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, TileCoordVisitor)
    }
}

impl Add for TileCoord {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for TileCoord {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<TileUnit> for TileCoord {
    type Output = Self;

    fn mul(self, rhs: TileUnit) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<TileUnit> for TileCoord {
    type Output = Self;

    fn div(self, rhs: TileUnit) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Neg for TileCoord {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
