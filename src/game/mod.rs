use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use flexstr::SharedStr;
use hexagon_tiles::traits::HexDirection;
use rand::distributions::Bernoulli;
use rand::distributions::Distribution;
use rand::Rng;
use riker::actor::{Actor, BasicActorRef};
use riker::actors::{ActorFactoryArgs, ActorRef, ActorRefFactory, Context, Sender, Strategy, Tell};
use uuid::Uuid;

use crate::game::map::{Map, RenderContext};
use crate::game::ticking::TickUnit;
use crate::game::tile::coord::{TileCoord, TileHex, TileUnit};
use crate::game::tile::entity::{Data, TileEntity, TileEntityMsg, TileState};
use crate::game::GameMsg::*;
use crate::resource::item::id_eq_or_of_tag;
use crate::resource::script::Script;
use crate::resource::ResourceManager;
use crate::util::id::Id;

/// Handles input.
pub mod input;
/// Handles the inventory of tiles.
pub mod inventory;
/// Defines items and moving them around.
pub mod item;
/// Defines the map.
pub mod map;
/// Starts and runs the game.
pub mod run;
/// Manages ticking the game state.
pub mod ticking;
/// Defines tiles and tile entities.
pub mod tile;

#[derive(Debug, Clone)]
pub struct Game {
    /// a count of all the ticks that have happened
    tick_count: TickUnit,

    /// the resource manager
    resource_man: Arc<ResourceManager>,

    /// the map
    map: Arc<Mutex<Map>>,

    /// is the game stopped
    stopped: bool,
    /// scheduled messages to be sent next tick
    next_tick_messages: HashMap<TileCoord, Vec<(TileEntityMsg, Option<BasicActorRef>)>>,
}

impl ActorFactoryArgs<(Arc<ResourceManager>, SharedStr)> for Game {
    fn create_args(args: (Arc<ResourceManager>, SharedStr)) -> Self {
        Self::new(args.0, args.1)
    }
}

/// Represents a message the game receives
#[derive(Debug, Clone)]
pub enum GameMsg {
    /// tick the tile once
    Tick,
    /// populate map
    Populate(TileCoord),
    /// get rendering information
    RenderInfoRequest {
        context: RenderContext,
    },
    /// place a tile at the given position
    PlaceTile {
        coord: TileCoord,
        id: Id,
        tile_state: TileState,
    },
    /// get the tile at the given position
    GetTile(TileCoord),
    /// send a message to a tile entity
    SendMsgToTile(TileCoord, TileEntityMsg),
    /// send a message to a tile entity next tick
    NextTickMsgToTile(TileCoord, TileEntityMsg, Option<BasicActorRef>),
    /// checks for the adjacent tiles against the script
    CheckAdjacent {
        script: Script,
        coord: TileCoord,
    },
    /// get the map
    GetMap,
    /// load a map
    LoadMap(Arc<ResourceManager>),
    GetData,
    GetDataValue(String),
    SetData(String, Data),
    RemoveData(String),
    Stop,
}

#[derive(Debug, Copy, Clone)]
pub enum PlaceTileResponse {
    Placed,
    Removed,
    Ignored,
}

pub const POPULATE_RANGE: TileUnit = 64;

impl Actor for Game {
    type Msg = GameMsg;

    fn supervisor_strategy(&self) -> Strategy {
        Strategy::Stop
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        let myself = Some(ctx.myself().into());

        match &msg {
            GetMap => {
                sender.inspect(|v| v.try_tell(self.map.clone(), myself).unwrap());
                return;
            }
            LoadMap(resource_man) => {
                let map = self.map.lock().unwrap();

                map.tiles.iter().for_each(|(_, (tile, _, _))| {
                    ctx.system.stop(tile);
                });

                let name = map.map_name.clone();

                drop(map);

                self.map = Arc::new(Mutex::new(Map::load(ctx, resource_man.clone(), name)));
                return;
            }
            GetData => {
                let map = self.map.lock().unwrap();

                sender.inspect(|v| v.try_tell(map.data.clone(), myself).unwrap());
                return;
            }
            GetDataValue(key) => {
                let map = self.map.lock().unwrap();

                sender.inspect(|v| {
                    v.try_tell(map.data.get(key.as_str()).cloned(), myself)
                        .unwrap()
                });
                return;
            }
            _ => {}
        }

        if self.stopped {
            return;
        }

        match msg {
            Populate(coord) => {
                let mut map = self.map.lock().unwrap();

                let key = coord.to_formal_string();

                if Some(&true) == map.data.get(&key).and_then(Data::as_bool) {
                    return;
                }

                self.populate(coord, ctx, &mut map);

                map.data.insert(key, Data::Bool(true));
            }
            Tick => {
                self.tick();
            }
            RenderInfoRequest { context } => {
                let render_info = self.map.lock().unwrap().render_info(context);

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

                if id == self.resource_man.registry.none {
                    if !map.tiles.contains_key(&coord) {
                        sender.inspect(|v| v.try_tell(PlaceTileResponse::Ignored, myself).unwrap());
                        return;
                    }

                    map.tiles.remove_entry(&coord);
                    sender.inspect(|v| v.try_tell(PlaceTileResponse::Removed, myself).unwrap());
                } else {
                    let tile = Self::new_tile(ctx, coord, id, tile_state);

                    map.tiles.insert(coord, (tile, id, tile_state));
                    sender.inspect(|v| v.try_tell(PlaceTileResponse::Placed, myself).unwrap());
                }

                map.render_cache.clear();
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
            NextTickMsgToTile(coord, msg, sender) => {
                self.next_tick_messages.entry(coord).or_insert_with(Default::default).push((msg, sender));
            }
            SetData(key, value) => {
                let mut map = self.map.lock().unwrap();

                map.data.insert(key, value);
            }
            RemoveData(key) => {
                let mut map = self.map.lock().unwrap();

                map.data.remove(&key);
            }
            CheckAdjacent { script, coord } => {
                if let Some(adjacent) = script.adjacent {
                    let map = self.map.lock().unwrap();

                    let mut fulfilled = false;

                    for neighbor in TileHex::NEIGHBORS.iter().map(|v| coord + (*v).into()) {
                        if let Some((_, id, _)) = map.tiles.get(&neighbor) {
                            if id_eq_or_of_tag(&self.resource_man.registry, *id, adjacent) {
                                fulfilled = true;
                                break;
                            }
                        }
                    }

                    sender.map(|v| v.try_tell(TileEntityMsg::AdjacentState { fulfilled }, myself));
                } else {
                    sender.map(|v| {
                        v.try_tell(TileEntityMsg::AdjacentState { fulfilled: true }, myself)
                    });
                }
            }
            Stop => {
                self.stopped = true;
            }
            _ => {}
        }
    }
}

impl Game {
    /// Creates a new game messaging/map system.
    pub fn new(resource_man: Arc<ResourceManager>, map_name: SharedStr) -> Self {
        Self {
            tick_count: 0,

            resource_man,

            map: Arc::new(Mutex::new(Map::new_empty(map_name.to_string()))),

            stopped: false,
            next_tick_messages: Default::default(),
        }
    }

    /// Creates a new tile of given type at the given position, and with an initial state.
    fn new_tile(
        ctx: &Context<GameMsg>,
        coord: TileCoord,
        id: Id,
        tile_state: TileState,
    ) -> ActorRef<TileEntityMsg> {
        ctx.system
            .actor_of_args::<TileEntity, (BasicActorRef, Id, TileCoord, TileState)>(
                &Uuid::new_v4().to_string(),
                (ctx.myself().into(), id, coord, tile_state),
            )
            .unwrap()
    }

    /// Populates the map.
    fn populate(&self, coord: TileCoord, ctx: &Context<GameMsg>, map: &mut Map) {
        let start = coord * POPULATE_RANGE;
        let start = (
            start.q() - POPULATE_RANGE / 2,
            start.r() - POPULATE_RANGE / 2,
        );

        let end = (start.0 + POPULATE_RANGE, start.1 + POPULATE_RANGE);

        let src = self.resource_man.registry.deposit_tiles();
        let range = 0..src.len();

        let mut rng = rand::thread_rng();
        let d = Bernoulli::new(0.005).unwrap();

        for q in start.0..end.0 {
            for r in start.1..end.1 {
                if d.sample(&mut rng) {
                    let coord = TileCoord::new(q, r);
                    let id = src[rng.gen_range(range.clone())];

                    map.tiles
                        .insert(coord, (Self::new_tile(ctx, coord, id, 0), id, 0));
                }
            }
        }
    }
}
