use std::sync::Arc;
use std::time::Instant;

use riker::actor::{Actor, BasicActorRef};
use riker::actors::{ActorFactoryArgs, ActorRefFactory, Context, Sender, Strategy, Tell};
use uuid::Uuid;

use crate::game::data::Data;
use crate::game::game::GameMsg::*;
use crate::game::map::{Map, RenderContext};
use crate::game::ticking::MAX_ALLOWED_TICK_INTERVAL;
use crate::game::tile::{StateUnit, TileCoord, TileEntity, TileEntityMsg};
use crate::util::id::Id;
use crate::util::resource::ResourceManager;

pub type TickUnit = u16;

#[derive(Debug, Clone)]
pub struct Ticked;

#[derive(Debug, Clone)]
pub struct Game {
    tick_count: TickUnit,

    resource_man: Arc<ResourceManager>,

    map: Map,
}

impl ActorFactoryArgs<(Arc<ResourceManager>, Arc<Map>)> for Game {
    fn create_args(args: (Arc<ResourceManager>, Arc<Map>)) -> Self {
        Self::new(args.0, Arc::unwrap_or_clone(args.1))
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
        none: Id,
        tile_state: StateUnit,
    },
    GetTile(TileCoord),
    SendMsgToTile(TileCoord, TileEntityMsg),
}

#[derive(Debug, Copy, Clone)]
pub enum PlaceTileResponse {
    Placed,
    Removed,
    Ignored,
}

impl Actor for Game {
    type Msg = GameMsg;

    fn supervisor_strategy(&self) -> Strategy {
        Strategy::Stop
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        let myself = Some(ctx.myself().into());

        match msg {
            Tick => {
                self.tick();
            }
            RenderInfoRequest { context } => {
                let render_info = self.map.render_info(&context);

                sender.inspect(|v| v.try_tell(render_info, myself).unwrap());
            }
            PlaceTile {
                coord,
                id,
                none,
                tile_state,
            } => {
                if let Some((old_id, tile, old_tile_state)) = self.map.tiles.get(&coord) {
                    if *old_tile_state == tile_state && *old_id == id {
                        sender.inspect(|v| v.try_tell(PlaceTileResponse::Ignored, myself).unwrap());
                        return;
                    }

                    ctx.system.stop(tile);
                }

                if id == none {
                    self.map.tiles.remove_entry(&coord);

                    sender.inspect(|v| v.try_tell(PlaceTileResponse::Removed, myself).unwrap());
                    return;
                }

                let tile = ctx
                    .system
                    .actor_of_args::<TileEntity, (BasicActorRef, Id, TileCoord, Data, StateUnit)>(
                        &Uuid::new_v4().to_string(),
                        (ctx.myself().into(), id, coord, Data::default(), tile_state),
                    )
                    .unwrap();

                self.map.tiles.insert(coord, (id, tile, tile_state));
                sender.inspect(|v| v.try_tell(PlaceTileResponse::Placed, myself).unwrap());
            }
            GetTile(coord) => {
                sender.inspect(|v| {
                    v.try_tell(self.map.tiles.get(&coord).cloned(), myself)
                        .unwrap();
                });
            }
            SendMsgToTile(coord, msg) => {
                if let Some((_, tile, _)) = self.map.tiles.get(&coord) {
                    tile.tell(msg, sender);
                }
            }
        }
    }
}

impl Game {
    pub fn new(resource_man: Arc<ResourceManager>, map: Map) -> Self {
        Self {
            tick_count: 0,

            resource_man,

            map,
        }
    }

    fn inner_tick(&mut self) {
        for (_, (_, tile, _)) in self.map.tiles.iter() {
            tile.tell(
                TileEntityMsg::Tick {
                    resource_man: self.resource_man.clone(),
                    tick_count: self.tick_count,
                },
                None,
            );
        }

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
