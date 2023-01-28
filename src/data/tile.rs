use std::fmt::{Display, Formatter};
use std::sync::Arc;
use hexagon_tiles::hex::{Hex, hex};
use riker::actor::{Context, Sender};
use riker::actors::{Actor, ActorFactoryArgs, BasicActorRef};

use super::data::Data;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use crate::data::id::{Id, id_static};
use crate::game::item::Item;
use crate::game::script::Script;
use crate::util::init::InitData;

pub const NONE: Id = id_static("automancy", "none");

#[derive(Debug, Clone)]
pub struct Tile {
    pub id: Id,
    pub data: Data,
    pub script: Option<Id>,
    pub target: TileCoord,
}

#[derive(Debug, Copy, Clone)]
pub enum TransactionError {
    NotEnough,
    NoScript,
    NotSuitable,
}

#[derive(Debug, Clone)]
pub enum TileMsg {
    GetId,
    Transaction {
        item: Item,
        init_data: Arc<InitData>,
    },
    TransactionResult(Result<Item, TransactionError>),
    SetTarget(TileCoord),
    GetTarget,
    SetScript(Id),
    GetScript,
}

impl Actor for Tile {
    type Msg = TileMsg;

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        let myself = Some(ctx.myself().into());

        match msg {
            TileMsg::GetId => {
                sender.inspect(|v| v.try_tell(self.id.clone(), myself).unwrap());
            }
            TileMsg::Transaction { item, init_data } => {
                if let Some(sender) = sender {
                    if let Some(script) = self.get_script(init_data) {
                        let input = script.instructions.input;
                        let id = item.id;
                        let has = item.amount;
                        let required = input.amount;

                        if input.id != id {
                            sender.try_tell(TileMsg::TransactionResult(Err(TransactionError::NotSuitable)), myself).unwrap();
                            return;
                        }

                        if required > has {
                            sender.try_tell(TileMsg::TransactionResult(Err(TransactionError::NotEnough)), myself).unwrap();
                            return;
                        }

                        let amount = self.data.0.entry(id.clone()).or_insert(0);
                        *amount += required;

                        sender.try_tell(TileMsg::TransactionResult(Ok(Item { id, amount: has - required })), myself).unwrap();
                    } else {
                        sender.try_tell(TileMsg::TransactionResult(Err(TransactionError::NoScript)), myself).unwrap();
                        return;
                    }
                }
            }
            TileMsg::TransactionResult(result) => {
                // TODO slow down if error
                if let Ok(new) = result {
                    let amount = self.data.0.entry(new.id).or_insert(0);
                    *amount = new.amount;
                }
            }
            TileMsg::SetTarget(target) => {
                self.target = target;
            }
            TileMsg::GetTarget => {
                sender.inspect(|v| v.try_tell(self.target, myself).unwrap());
            }
            TileMsg::SetScript(id) => {
                self.script = Some(id)
            }
            TileMsg::GetScript => {
                sender.inspect(|v| v.try_tell(self.script.clone(), myself).unwrap());
            }
        }
    }
}

impl ActorFactoryArgs<(Id, Data)> for Tile {
    fn create_args(args: (Id, Data)) -> Self {
        Self::new(args.0, args.1)
    }
}

impl Tile {
    fn new(id: Id, data: Data) -> Self {
        Self {
            id,
            data,
            script: None,
            target: TileCoord::ZERO,
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