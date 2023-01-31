use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::sync::Arc;

use egui::NumExt;
use hexagon_tiles::hex::{Hex, hex};
use hexagon_tiles::traits::HexDirection;
use riker::actor::{Context, Sender};
use riker::actors::{Actor, ActorFactoryArgs, BasicActorRef};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;

use crate::data::data::Data;
use crate::data::id::{Id, id_static, IdRaw};
use crate::game::game::GameMsg;
use crate::game::item::Item;
use crate::game::script::Script;
use crate::util::init::InitData;
use crate::util::resource::ResourceType;

#[derive(Debug, Clone)]
pub struct Tile {
    id: Id,
    coord: TileCoord,
    data: Data,
    script: Option<Id>,

    game: BasicActorRef,
    target_direction: Option<Hex<TileUnit>>,

    interval_offset: usize,
}

#[derive(Debug, Copy, Clone)]
pub enum TransactionError {
    NotEnough,
    NoScript,
    NotSuitable,
    Full,
}

#[derive(Debug, Clone)]
pub enum TileMsg {
    Tick {
        init_data: Arc<InitData>,
        tick_count: usize,
    },
    Transaction {
        item: Item,
        tick_count: usize,
        source_type: ResourceType,
        direction: Hex<TileUnit>,
        init_data: Arc<InitData>,
    },
    TransactionResult(Result<(), TransactionError>),
    SetTarget(Option<Hex<TileUnit>>),
    GetTarget,
    SetScript(Id),
    GetScript,
    GetData,
}

const L_SPLITTER: IdRaw = id_static("automancy", "l_splitter");
const L_SPLITTER_A: Hex<TileUnit> = Hex::<TileUnit>::NEIGHBORS[1];
const L_SPLITTER_B: Hex<TileUnit> = Hex::<TileUnit>::NEIGHBORS[3];
const L_SPLITTER_C: Hex<TileUnit> = Hex::<TileUnit>::NEIGHBORS[5];

const R_SPLITTER: IdRaw = id_static("automancy", "r_splitter");
const R_SPLITTER_A: Hex<TileUnit> = Hex::<TileUnit>::NEIGHBORS[0];
const R_SPLITTER_B: Hex<TileUnit> = Hex::<TileUnit>::NEIGHBORS[2];
const R_SPLITTER_C: Hex<TileUnit> = Hex::<TileUnit>::NEIGHBORS[4];

impl Actor for Tile {
    type Msg = TileMsg;

    fn post_stop(&mut self) {
        self.script = None;
        self.target_direction = None;
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        let myself = Some(ctx.myself().into());

        match msg {
            TileMsg::Tick { init_data, tick_count } => {
                let interval = 1 + self.interval_offset;

                if (tick_count % interval) != 0 {
                    return;
                }

                let resource_type = init_data.resource_man.resources[&self.id].resource_type.clone();

                match resource_type {
                    ResourceType::Machine(_) => {
                        self.machine_tell(myself, init_data, resource_type, tick_count);
                    }
                    _ => {
                    }
                }
            }
            TileMsg::Transaction { item, tick_count, source_type, direction, init_data } => {
                if let Some(sender) = sender {
                    let resource_type = init_data.resource_man.resources[&self.id].resource_type.clone();

                    match &resource_type {
                        ResourceType::Machine(_) => {
                            self.machine_result(myself, sender, init_data, item);
                        }
                        ResourceType::Transfer(id) => {
                            if let ResourceType::Transfer(_) = source_type {
                                return;
                            }

                            if id == &L_SPLITTER {
                                let (a, b) = match -direction {
                                    L_SPLITTER_A => {
                                        (L_SPLITTER_B, L_SPLITTER_C)
                                    }
                                    L_SPLITTER_B => {
                                        (L_SPLITTER_A, L_SPLITTER_C)
                                    }
                                    L_SPLITTER_C => {
                                        (L_SPLITTER_A, L_SPLITTER_B)
                                    }
                                    _ => {
                                        return;
                                    }
                                };

                                let target = if tick_count % 2 == 0 {
                                    a
                                } else {
                                    b
                                };

                                let coord = TileCoord(self.coord.0 + target);

                                self.game.try_tell(GameMsg::SendMsgToTile(coord, TileMsg::Transaction { item, tick_count, source_type: resource_type.clone(), direction: target, init_data }), Some(sender)).unwrap();
                            } else if id == &R_SPLITTER {
                                let (a, b) = match -direction {
                                    R_SPLITTER_A => {
                                        (R_SPLITTER_B, R_SPLITTER_C)
                                    }
                                    R_SPLITTER_B => {
                                        (R_SPLITTER_A, R_SPLITTER_C)
                                    }
                                    R_SPLITTER_C => {
                                        (R_SPLITTER_A, R_SPLITTER_B)
                                    }
                                    _ => {
                                        return;
                                    }
                                };

                                let target = if tick_count % 2 == 0 {
                                    a
                                } else {
                                    b
                                };

                                let coord = TileCoord(self.coord.0 + target);

                                self.game.try_tell(GameMsg::SendMsgToTile(coord, TileMsg::Transaction { item, tick_count, source_type: resource_type.clone(), direction: target, init_data }), Some(sender)).unwrap();
                            }
                        }
                        ResourceType::Void => {
                            sender.try_tell(TileMsg::TransactionResult(Ok(())), myself).unwrap();
                        }
                        _ => {
                            sender.try_tell(TileMsg::TransactionResult(Err(TransactionError::NotSuitable)), myself).unwrap();
                        }
                    }
                }
            }
            TileMsg::TransactionResult(result) => {
                match result {
                    Ok(_) => {
                        self.interval_offset = 0;
                    }
                    Err(error) => {
                        match error {
                            TransactionError::NoScript | TransactionError::NotSuitable | TransactionError::Full => {
                                self.slow_down();
                                return;
                            }
                            TransactionError::NotEnough => {
                                self.interval_offset = 0;
                            }
                        }
                    }
                }
            }
            TileMsg::SetTarget(target_direction) => {
                self.target_direction = target_direction;
            }
            TileMsg::GetTarget => {
                sender.inspect(|v| v.try_tell(self.target_direction.clone(), myself).unwrap());
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

impl ActorFactoryArgs<(BasicActorRef, Id, TileCoord, Data)> for Tile {
    fn create_args(args: (BasicActorRef, Id, TileCoord, Data)) -> Self {
        Self::new(args.0, args.1, args.2, args.3)
    }
}

impl Tile {
    fn machine_tell(&mut self, myself: Option<BasicActorRef>, init_data: Arc<InitData>, resource_type: ResourceType, tick_count: usize) {
        if let Some(direction) = self.target_direction {
            let coord = TileCoord(self.coord.0 + direction);

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
                            self.send_tile_msg(myself, coord, TileMsg::Transaction { item: output, tick_count, source_type: resource_type.clone(), direction, init_data });
                        }
                    }
                } else {
                    if let Some(output) = output {
                        self.send_tile_msg(myself, coord, TileMsg::Transaction { item: output, tick_count, source_type: resource_type.clone(), direction, init_data });
                    }
                }
            }
        } else {
            self.slow_down();
        }
    }

    fn machine_result(&mut self, myself: Option<BasicActorRef>, sender: BasicActorRef, init_data: Arc<InitData>, item: Item) {
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

                let limit = required * 2;

                if *amount > limit {
                    *amount = amount.at_most(limit);

                    sender.try_tell(TileMsg::TransactionResult(Err(TransactionError::Full)), myself).unwrap();
                    return;
                }

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

    fn slow_down(&mut self) {
        self.interval_offset += 2;
        self.interval_offset = self.interval_offset.at_most(9);
    }

    fn send_tile_msg(&mut self, myself: Option<BasicActorRef>, coord: TileCoord, msg: TileMsg) {
        match self.game.try_tell(GameMsg::SendMsgToTile(coord, msg), myself) {
            Ok(_) => {}
            Err(_) => {
                self.target_direction = None;
            }
        }
    }

    fn new(game: BasicActorRef, id: Id, coord: TileCoord, data: Data) -> Self {
        Self {
            id,
            coord,
            data,
            script: None,

            game,
            target_direction: None,

            interval_offset: 0,
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