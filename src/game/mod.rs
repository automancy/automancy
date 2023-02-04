use flexstr::SharedStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use riker::actor::{Actor, BasicActorRef};
use riker::actors::{ActorFactoryArgs, ActorRef, ActorRefFactory, Context, Sender, Strategy, Tell};
use uuid::Uuid;

use crate::game::data::{Data, TileCoord};
use crate::game::map::{Map, RenderContext};
use crate::game::ticking::MAX_ALLOWED_TICK_INTERVAL;
use crate::game::tile::{StateUnit, TileEntity, TileEntityMsg};
use crate::game::GameMsg::*;
use crate::resource::ResourceManager;
use crate::util::id::Id;

pub mod data;
pub mod input;
pub mod item;
pub mod map;
pub mod run;
pub mod ticking;
pub mod tile;

pub type TickUnit = u16;

#[derive(Debug, Clone)]
pub struct Ticked;

#[derive(Debug, Clone)]
pub struct Game {
    tick_count: TickUnit,

    resource_man: Arc<ResourceManager>,

    map: Arc<Mutex<Map>>,
}

impl ActorFactoryArgs<(Arc<ResourceManager>, SharedStr)> for Game {
    fn create_args(args: (Arc<ResourceManager>, SharedStr)) -> Self {
        Self::new(args.0, args.1)
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
        tile_state: StateUnit,
    },
    GetTile(TileCoord),
    SendMsgToTile(TileCoord, TileEntityMsg),
    GetMap,
    LoadMap(Arc<ResourceManager>),
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
                let render_info = self.map.lock().unwrap().render_info(&context);

                sender.inspect(|v| v.try_tell(render_info, myself).unwrap());
            }
            PlaceTile {
                coord,
                id,
                tile_state,
            } => {
                let mut map = self.map.lock().unwrap();

                if let Some((tile, old_id, old_tile_state)) = map.tiles.get(&coord) {
                    if *old_tile_state == tile_state && *old_id == id {
                        sender.inspect(|v| v.try_tell(PlaceTileResponse::Ignored, myself).unwrap());
                        return;
                    }

                    ctx.system.stop(tile);
                }

                if id == self.resource_man.none {
                    if !map.tiles.contains_key(&coord) {
                        sender.inspect(|v| v.try_tell(PlaceTileResponse::Ignored, myself).unwrap());
                        return;
                    }

                    map.tiles.remove_entry(&coord);
                    sender.inspect(|v| v.try_tell(PlaceTileResponse::Removed, myself).unwrap());
                    return;
                }
                let tile = Self::new_tile(&ctx, coord, id, tile_state);

                map.tiles.insert(coord, (tile, id, tile_state));
                sender.inspect(|v| v.try_tell(PlaceTileResponse::Placed, myself).unwrap());
            }
            GetTile(coord) => {
                sender.inspect(|v| {
                    v.try_tell(self.map.lock().unwrap().tiles.get(&coord).cloned(), myself)
                        .unwrap();
                });
            }
            SendMsgToTile(coord, msg) => {
                if let Some((tile, _, _)) = self.map.lock().unwrap().tiles.get(&coord) {
                    tile.tell(msg, sender);
                }
            }
            GetMap => {
                sender.inspect(|v| v.try_tell(self.map.clone(), myself).unwrap());
            }
            LoadMap(resource_man) => {
                let map = self.map.lock().unwrap();

                map.tiles.iter().for_each(|(_, (tile, _, _))| {
                    ctx.system.stop(tile);
                });

                let name = map.map_name.clone();

                drop(map);

                self.map = Arc::new(Mutex::new(Map::load(ctx, &resource_man.interner, name)));
            }
        }
    }
}

impl Game {
    pub fn new(resource_man: Arc<ResourceManager>, map_name: SharedStr) -> Self {
        Self {
            tick_count: 0,

            resource_man,

            map: Arc::new(Mutex::new(Map::new_empty(map_name.to_string()))),
        }
    }

    fn new_tile(
        ctx: &Context<GameMsg>,
        coord: TileCoord,
        id: Id,
        tile_state: StateUnit,
    ) -> ActorRef<TileEntityMsg> {
        ctx.system
            .actor_of_args::<TileEntity, (BasicActorRef, Id, TileCoord, Data, StateUnit)>(
                &Uuid::new_v4().to_string(),
                (ctx.myself().into(), id, coord, Data::default(), tile_state),
            )
            .unwrap()
    }

    fn inner_tick(&mut self) {
        for (_, (tile, _, _)) in self.map.lock().unwrap().tiles.iter() {
            tile.send_msg(
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
