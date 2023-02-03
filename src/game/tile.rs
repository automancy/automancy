use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::sync::Arc;

use egui::NumExt;
use hexagon_tiles::hex::{hex, Hex};
use hexagon_tiles::traits::HexDirection;
use riker::actor::{Context, Sender};
use riker::actors::{Actor, ActorFactoryArgs, BasicActorRef};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::game::data::Data;
use crate::game::game::GameMsg;
use crate::game::item::Item;
use crate::game::script::Script;
use crate::game::tile::TileEntityMsg::*;
use crate::util::id::{id_static, Id, IdRaw};
use crate::util::resource::ResourceManager;
use crate::util::resource::TileType;
use crate::util::resource::TileType::*;

#[derive(Debug, Clone)]
pub struct TileEntity {
    id: Id,
    coord: TileCoord,
    data: Data,
    script: Option<Id>,

    game: BasicActorRef,
    target_direction: Option<Hex<TileUnit>>,

    interval_offset: usize,

    tile_state: usize,
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
        tick_count: usize,
    },
    Transaction {
        item: Item,
        tick_count: usize,
        source_type: TileType,
        direction: Hex<TileUnit>,
        resource_man: Arc<ResourceManager>,
    },
    TransactionResult(Result<(), TransactionError>),

    SetTarget(Option<Hex<TileUnit>>),
    GetTarget,

    SetScript(Id),
    GetScript,

    GetData,
}

const SPLITTER: IdRaw = id_static("automancy", "splitter");

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
                item,
                tick_count,
                source_type,
                direction,
                resource_man,
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

                            if id == &SPLITTER {
                                let in_dir = -direction;

                                let a: Hex<TileUnit> =
                                    Hex::<TileUnit>::NEIGHBORS[1 - self.tile_state];
                                let b: Hex<TileUnit> =
                                    Hex::<TileUnit>::NEIGHBORS[3 - self.tile_state];
                                let c: Hex<TileUnit> =
                                    Hex::<TileUnit>::NEIGHBORS[5 - self.tile_state];

                                let (first, second) = if in_dir == a {
                                    (b, c)
                                } else if in_dir == b {
                                    (a, c)
                                } else if in_dir == c {
                                    (a, b)
                                } else {
                                    return;
                                };

                                let target = if tick_count % 2 == 0 { first } else { second };

                                let coord = TileCoord(self.coord.0 + target);

                                self.game
                                    .try_tell(
                                        GameMsg::SendMsgToTile(
                                            coord,
                                            Transaction {
                                                item,
                                                tick_count,
                                                source_type: tile_type.clone(),
                                                direction: target,
                                                resource_man,
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

impl ActorFactoryArgs<(BasicActorRef, Id, TileCoord, Data, usize)> for TileEntity {
    fn create_args(args: (BasicActorRef, Id, TileCoord, Data, usize)) -> Self {
        Self::new(args.0, args.1, args.2, args.3, args.4)
    }
}

impl TileEntity {
    fn machine_tell(
        &mut self,
        myself: Option<BasicActorRef>,
        resource_man: Arc<ResourceManager>,
        tile_type: TileType,
        tick_count: usize,
    ) {
        if let Some(direction) = self.target_direction {
            let coord = TileCoord(self.coord.0 + direction);

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
                                    item: output,
                                    tick_count,
                                    source_type: tile_type,
                                    direction,
                                    resource_man,
                                },
                            );
                        }
                    }
                } else if let Some(output) = output {
                    self.send_tile_msg(
                        myself,
                        coord,
                        Transaction {
                            item: output,
                            tick_count,
                            source_type: tile_type,
                            direction,
                            resource_man,
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

    fn new(game: BasicActorRef, id: Id, coord: TileCoord, data: Data, tile_state: usize) -> Self {
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
