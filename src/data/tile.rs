use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::sync::Arc;

use egui::NumExt;
use hexagon_tiles::hex::{Hex, hex};
use riker::actor::{Context, Sender};
use riker::actors::{Actor, ActorFactoryArgs, BasicActorRef};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;

use crate::data::id::Id;
use crate::game::item::Item;
use crate::game::script::Script;
use crate::util::init::InitData;

use super::data::Data;

#[derive(Debug, Clone)]
pub struct Tile {
    pub data: Data,
    pub script: Option<Id>,
    pub target_coord: Option<TileCoord>,
    pub target_ref: Option<BasicActorRef>,
}

#[derive(Debug, Copy, Clone)]
pub enum TransactionError {
    NotEnough,
    NoScript,
    NotSuitable,
}

#[derive(Debug, Clone)]
pub enum TileMsg {
    Tick {
        init_data: Arc<InitData>,
    },
    Transaction {
        item: Item,
        init_data: Arc<InitData>,
    },
    TransactionResult(Result<(), TransactionError>),
    SetTarget((TileCoord, BasicActorRef)),
    GetTarget,
    SetScript(Id),
    GetScript,
    GetData,
}

impl Actor for Tile {
    type Msg = TileMsg;

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        let myself = Some(ctx.myself().into());

        match msg {
            TileMsg::Tick { init_data } => {
                if let Some(target) = self.target_ref.clone() {
                    if let Some(script) = self.script.as_ref().and_then(|v| init_data.resource_man.scripts.get(v)) {
                        let instructions = &script.instructions;
                        let output = instructions.output.clone();

                        if let Some(input) = instructions.input.clone() {
                            let id = input.id;

                            // TODO send transaction result back to Game
                            let stored = *self.data.0.get(&id).unwrap_or(&0);
                            if stored >= input.amount {
                                self.data.0.insert(id, stored - input.amount);

                                if let Some(output) = output {
                                    target.try_tell(TileMsg::Transaction { item: output, init_data }, myself).unwrap();
                                }
                            }
                        } else {
                            if let Some(output) = output {
                                target.try_tell(TileMsg::Transaction { item: output, init_data }, myself).unwrap();
                            }
                        }
                    }
                }
            }
            TileMsg::Transaction { item, init_data } => {
                if let Some(sender) = sender {
                    if let Some(script) = self.get_script(init_data) {
                        if let Some(input) = script.instructions.input {
                            let id = item.id;
                            let has = item.amount;
                            let required = input.amount;

                            if input.id != id {
                                sender.try_tell(TileMsg::TransactionResult(Err(TransactionError::NotSuitable)), myself).unwrap();
                                return;
                            }

                            let amount = self.data.0.entry(id.clone()).or_insert(0);
                            *amount += has;
                            *amount = amount.at_most(required * 2);

                            if has >= required {
                                sender.try_tell(TileMsg::TransactionResult(Ok(())), myself).unwrap();
                                return;
                            } else {
                                sender.try_tell(TileMsg::TransactionResult(Err(TransactionError::NotEnough)), myself).unwrap();
                                return;
                            }
                        } else {
                            sender.try_tell(TileMsg::TransactionResult(Err(TransactionError::NotSuitable)), myself).unwrap();
                            return;
                        }
                    } else {
                        sender.try_tell(TileMsg::TransactionResult(Err(TransactionError::NoScript)), myself).unwrap();
                        return;
                    }
                }
            }
            TileMsg::TransactionResult(_result) => {
                // TODO slow down if error
            }
            TileMsg::SetTarget((target_coord, target_ref)) => {
                self.target_coord = Some(target_coord);
                self.target_ref = Some(target_ref);
            }
            TileMsg::GetTarget => {
                sender.inspect(|v| v.try_tell((self.target_coord, self.target_ref.clone()), myself).unwrap());
            }
            TileMsg::SetScript(id) => {
                self.script = Some(id);
                self.data.0.clear();
            }
            TileMsg::GetScript => {
                sender.inspect(|v| v.try_tell(self.script.clone(), myself).unwrap());
            }
            TileMsg::GetData => {
                sender.inspect(|v| v.try_tell(self.data.clone(), myself).unwrap());
            }
        }
    }
}

impl ActorFactoryArgs<Data> for Tile {
    fn create_args(args: Data) -> Self {
        Self::new(args)
    }
}

impl Tile {
    fn new(data: Data) -> Self {
        Self {
            data,
            script: None,
            target_coord: None,
            target_ref: None,
        }
    }

    fn get_script(&self, init_data: Arc<InitData>) -> Option<Script> {
        self.script.as_ref().and_then(|v| init_data.resource_man.scripts.get(v)).cloned()
    }
}

pub type TileUnit = i32;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TileCoord(pub Hex<TileUnit>);

impl TileCoord {
    pub const ZERO: Self = TileCoord(hex(0, 0, 0));

    pub fn new(q: TileUnit, r: TileUnit) -> Self {
        Self(Hex::new(q, r))
    }
}

impl Add for TileCoord {
    type Output = TileCoord;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
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
        where A: SeqAccess<'de>
    {
        let q: TileUnit = seq.next_element()?.unwrap();
        let r: TileUnit = seq.next_element()?.unwrap();

        Ok(TileCoord::new(q, r))
    }
}

impl<'de> Deserialize<'de> for TileCoord
    where
        Self: Sized
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>
    {
        deserializer.deserialize_tuple(2, TileCoordVisitor)
    }
}