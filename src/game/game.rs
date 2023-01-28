use std::time::Instant;
use riker::actor::{Actor, BasicActorRef};

use riker::actors::{Context, Sender, ActorFactoryArgs, ActorSystem, Tell, ActorRefFactory};
use uuid::Uuid;
use crate::data::data::Data;
use crate::data::map::{MapRenderInfo, RenderContext};

use crate::game::ticking::{MAX_ALLOWED_TICK_INTERVAL};

use crate::data::map::Map;
use crate::data::tile::{Tile, TileCoord, TileMsg};
use crate::data::id::Id;
use crate::data::tile;


#[derive(Debug, Clone)]
pub struct Ticked;



#[derive(Debug, Clone, Copy)]
pub struct GameState {
    pub tick_count: usize,
}

pub struct Game {
    tick_count: usize,

    map: Map,
}

impl ActorFactoryArgs<Map> for Game { // TODO dont clone Map
    fn create_args(args: Map) -> Self {
        Self::new(args)
    }
}

#[derive(Debug, Clone)]
pub enum GameMsg {
    Tick,
    RenderInfoRequest {
        context: RenderContext,
    },
    PlaceTile {
        coord: TileCoord,
        id: Id,
    },
    SendTileMsgTo {
        msg: TileMsg,
        to: TileCoord,
    },
    GetTile(TileCoord),
}

impl Actor for Game {
    type Msg = GameMsg;

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        let myself = Some(ctx.myself().into());

        match msg {
            GameMsg::Tick { .. } => {
                self.tick();
            }
            GameMsg::RenderInfoRequest { context } => {
                let render_info = self.map.render_info(&context);

                sender.inspect(|v| v.try_tell(render_info, myself).unwrap());
            }
            GameMsg::PlaceTile { coord, id } => {
                if id == tile::NONE {
                    self.map.tiles.remove_entry(&coord);
                } else {
                    let tile = ctx.system.actor_of_args::<Tile, (Id, Data)>(&Uuid::new_v4().to_string(), (id.clone(), Data::default()));

                    self.map.tiles.insert(coord, (id, tile.unwrap()));
                }
            }
            GameMsg::GetTile(coord) => {
                sender.inspect(|v| {
                    v.try_tell(self.map.tiles.get(&coord).cloned(), myself).unwrap();
                });
            }
            GameMsg::SendTileMsgTo { msg, to } => {
                if let Some((_, tile)) = self.map.tiles.get(&to) {
                    tile.tell(msg, myself);
                }
            }
        }
    }
}

impl Game {
    pub fn new(
        map: Map,
    ) -> Self {
        Self {
            tick_count: 0,

            map,
        }
    }

    fn inner_tick(&mut self) {
        self.tick_count = self.tick_count.overflowing_add(1).0;
    }

    pub fn tick(&mut self) {
        let start = Instant::now();
        self.inner_tick();
        let finish = Instant::now();

        let tick_time = finish - start;

        if tick_time >= MAX_ALLOWED_TICK_INTERVAL {
            log::warn!(
                "tick took longer than allowed maximum! tick_time: {:?}, maximum: {:?}",
                tick_time,
                MAX_ALLOWED_TICK_INTERVAL
            );
        }
    }
}
