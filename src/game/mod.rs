use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

use cgmath::vec3;
use flexstr::SharedStr;
use hexagon_tiles::layout::hex_to_pixel;
use hexagon_tiles::traits::HexDirection;
use rand::distributions::Bernoulli;
use rand::distributions::Distribution;
use rand::Rng;
use riker::actor::{Actor, BasicActorRef};
use riker::actors::{ActorFactoryArgs, ActorRef, ActorRefFactory, Context, Sender, Strategy};
use uuid::Uuid;

use crate::game::map::{Map, TileEntities};
use crate::game::ticking::TickUnit;
use crate::game::tile::coord::{ChunkCoord, TileCoord, TileHex, TileUnit};
use crate::game::tile::entity::{Data, TileEntity, TileEntityMsg, TileState};
use crate::game::GameMsg::*;
use crate::render::camera::FAR;
use crate::render::data::{InstanceData, HEX_GRID_LAYOUT};
use crate::resource::item::id_eq_or_of_tag;
use crate::resource::script::Script;
use crate::resource::ResourceManager;
use crate::util::cg::Float;
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

const UNDO_CACHE_SIZE: usize = 16;

#[derive(Debug, Clone)]
pub struct RenderUnit {
    pub instance: InstanceData,
    pub tile: Id,
    pub model: Id,
}

pub type RenderInfo = HashMap<TileCoord, RenderUnit>;

#[derive(Debug, Clone)]
pub struct Game {
    /// a count of all the ticks that have happened
    tick_count: TickUnit,

    /// the resource manager
    resource_man: Arc<ResourceManager>,

    /// the tile entities
    tile_entities: TileEntities,

    /// the map
    map: Map,

    /// render cache
    pub render_cache: HashMap<(TileUnit, TileCoord), Arc<RenderInfo>>,
    /// is the game stopped
    stopped: bool,
    /// what to do to undo the last 16 user events
    undo_steps: [Option<Vec<GameMsg>>; UNDO_CACHE_SIZE],
    /// the current index of the undo steps
    undo_steps_index: usize,
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
    Populate(ChunkCoord),
    /// get rendering information
    RenderInfoRequest {
        center: TileCoord,
        range: TileUnit,
    },
    /// place a tile at the given position
    PlaceTile {
        coord: TileCoord,
        id: Id,
        tile_state: TileState,
        record: bool,
    },
    Undo,
    /// get the tile at the given position
    GetTile(TileCoord),
    /// get the tile entity at the given position
    GetTileEntity(TileCoord),
    /// send a message to a tile entity
    ForwardMsgToTile(TileCoord, TileEntityMsg),
    /// checks for the adjacent tiles against the script
    CheckAdjacent {
        script: Script,
        coord: TileCoord,
    },
    TakeTileEntities,
    /// get the map
    TakeMap,
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

impl Actor for Game {
    type Msg = GameMsg;

    fn supervisor_strategy(&self) -> Strategy {
        Strategy::Stop
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        let myself = Some(ctx.myself().into());

        match &msg {
            TakeTileEntities => {
                if let Some(sender) = sender {
                    sender
                        .try_tell(mem::take(&mut self.tile_entities), myself)
                        .unwrap();
                }
                return;
            }
            TakeMap => {
                let map_name = self.map.map_name.clone();

                if let Some(sender) = sender {
                    sender
                        .try_tell(
                            mem::replace(&mut self.map, Map::new_empty(map_name)),
                            myself,
                        )
                        .unwrap();
                }
                return;
            }
            LoadMap(resource_man) => {
                self.tile_entities.values().for_each(|tile_entity| {
                    ctx.system.stop(tile_entity);
                });

                let name = self.map.map_name.clone();

                let (map, tile_entities) = Map::load(ctx, resource_man, name);

                self.map = map;
                self.tile_entities = tile_entities;

                return;
            }
            GetData => {
                if let Some(sender) = sender {
                    sender.try_tell(self.map.data.clone(), myself).unwrap();
                }
                return;
            }
            GetDataValue(key) => {
                if let Some(sender) = sender {
                    sender
                        .try_tell(self.map.data.get(key.as_str()).cloned(), myself)
                        .unwrap();
                }
                return;
            }
            _ => {}
        }

        if self.stopped {
            return;
        }

        match msg {
            Populate(coord) => {
                let key = coord.to_minimal_string();

                if Some(&true) == self.map.data.get(&key).and_then(Data::as_bool) {
                    return;
                }

                self.populate(coord, ctx);

                self.map.data.insert(key, Data::Bool(true));
            }
            Tick => {
                self.tick();
            }
            RenderInfoRequest { center, range } => {
                let render_info = self.render_info(center, range);

                if let Some(sender) = sender {
                    sender.try_tell(render_info, myself).unwrap();
                }
            }
            PlaceTile {
                coord,
                id,
                tile_state,
                record,
            } => {
                if let Some(((old_id, old_tile_state), old_tile_entity)) = self
                    .map
                    .tiles
                    .get(&coord)
                    .zip(self.tile_entities.get(&coord))
                {
                    if *old_tile_state == tile_state && *old_id == id {
                        if let Some(sender) = sender {
                            sender.try_tell(PlaceTileResponse::Ignored, myself).unwrap();
                        }
                        return;
                    }

                    ctx.system.stop(old_tile_entity);
                }

                let old_tile = if id == self.resource_man.registry.none {
                    if !self.map.tiles.contains_key(&coord) {
                        if let Some(sender) = sender {
                            sender.try_tell(PlaceTileResponse::Ignored, myself).unwrap();
                        }
                        return;
                    }

                    if let Some(sender) = sender {
                        sender.try_tell(PlaceTileResponse::Removed, myself).unwrap();
                    }

                    self.remove_tile(ctx, coord)
                } else {
                    if let Some(sender) = sender {
                        sender.try_tell(PlaceTileResponse::Placed, myself).unwrap();
                    }

                    self.insert_new_tile(ctx, coord, id, tile_state)
                };

                if record {
                    let (id, tile_state) = old_tile.unwrap_or((self.resource_man.registry.none, 0));

                    self.add_undo_step(vec![PlaceTile {
                        coord,
                        id,
                        tile_state,
                        record: false,
                    }])
                }

                self.render_cache.clear();
            }
            GetTile(coord) => {
                if let Some(sender) = sender {
                    sender
                        .try_tell(self.map.tiles.get(&coord).cloned(), myself)
                        .unwrap();
                }
            }
            GetTileEntity(coord) => {
                if let Some(sender) = sender {
                    sender
                        .try_tell(self.tile_entities.get(&coord).cloned(), myself)
                        .unwrap();
                };
            }
            ForwardMsgToTile(coord, msg) => {
                if let Some(tile_entity) = self.tile_entities.get(&coord) {
                    tile_entity.send_msg(msg, sender);
                }
            }
            SetData(key, value) => {
                self.map.data.insert(key, value);
            }
            RemoveData(key) => {
                self.map.data.remove(&key);
            }
            CheckAdjacent { script, coord } => {
                if let Some(adjacent) = script.adjacent {
                    let mut fulfilled = false;

                    for neighbor in TileHex::NEIGHBORS.iter().map(|v| coord + (*v).into()) {
                        if let Some((id, _)) = self.map.tiles.get(&neighbor) {
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
            Undo => {
                println!("{:?}", &self.undo_steps);
                self.undo_once(ctx.myself());
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

            tile_entities: Default::default(),
            map: Map::new_empty(map_name.to_string()),

            render_cache: Default::default(),
            stopped: false,
            undo_steps: Default::default(),
            undo_steps_index: 0,
        }
    }

    /// Removes a tile from both the map and the game
    pub fn remove_tile(
        &mut self,
        ctx: &Context<GameMsg>,
        coord: TileCoord,
    ) -> Option<(Id, TileState)> {
        if let Some(tile_entity) = self.tile_entities.get(&coord) {
            ctx.system.stop(tile_entity);
        }

        self.tile_entities.remove(&coord);

        self.map.tiles.remove(&coord)
    }

    /// Makes a new tile and add it into both the map and the game
    pub fn insert_new_tile(
        &mut self,
        ctx: &Context<GameMsg>,
        coord: TileCoord,
        id: Id,
        tile_state: TileState,
    ) -> Option<(Id, TileState)> {
        let tile_entity = Self::new_tile(ctx, coord, id, tile_state);

        self.tile_entities.insert(coord, tile_entity);

        self.map.tiles.insert(coord, (id, tile_state))
    }

    /// Undoes the last undo-able action with step stored in the undo steps array
    pub fn undo_once(&mut self, myself: ActorRef<GameMsg>) {
        if self.undo_steps.iter().all(Option::is_none) {
            return;
        }

        if self.undo_steps_index == 0 {
            self.undo_steps_index = UNDO_CACHE_SIZE - 1;
        } else {
            self.undo_steps_index -= 1;
        }

        if let Some(step) = mem::take(self.undo_steps.get_mut(self.undo_steps_index).unwrap()) {
            for msg in step {
                myself.send_msg(msg, None);
            }
        }

        println!("{:?}", self.undo_steps);
    }

    /// Adds one vector of undo steps to the undo steps array
    pub fn add_undo_step(&mut self, step: Vec<GameMsg>) {
        self.undo_steps[self.undo_steps_index] = Some(step);

        if self.undo_steps_index == UNDO_CACHE_SIZE - 1 {
            self.undo_steps_index = 0;
        } else {
            self.undo_steps_index += 1;
        }
    }

    /// Creates a new tile of given type at the given position, and with an initial state.
    pub fn new_tile(
        ctx: &Context<GameMsg>,
        coord: TileCoord,
        id: Id,
        tile_state: TileState,
    ) -> ActorRef<TileEntityMsg> {
        ctx.system
            .actor_of_args::<TileEntity, (BasicActorRef, Id, TileCoord, TileState)>(
                Uuid::new_v4().to_string().as_str(),
                (ctx.myself().into(), id, coord, tile_state),
            )
            .unwrap()
    }

    /// Populates the map.
    fn populate(&mut self, coord: ChunkCoord, ctx: &Context<GameMsg>) {
        let src = self.resource_man.registry.deposit_tiles();
        let range = 0..src.len();

        let mut rng = rand::thread_rng();
        let d = Bernoulli::new(0.005).unwrap();

        coord.iter().for_each(|coord| {
            if d.sample(&mut rng) {
                let id = src[rng.gen_range(range.clone())];

                self.map.tiles.insert(coord, (id, 0));
                self.tile_entities
                    .insert(coord, Self::new_tile(ctx, coord, id, 0));
            }
        });
    }

    pub fn render_info(&mut self, center: TileCoord, range: TileUnit) -> Arc<RenderInfo> {
        if let Some(info) = self.render_cache.get(&(range, center)) {
            return info.clone();
        }

        let instances: RenderInfo = self
            .map
            .tiles
            .iter()
            .filter(|(coord, _)| center.distance(**coord) <= range)
            .flat_map(|(coord, (id, tile_state))| {
                self.resource_man
                    .registry
                    .get_tile(id)
                    .and_then(|r| r.models.get(*tile_state as usize).cloned())
                    .map(|model| {
                        let p = hex_to_pixel(HEX_GRID_LAYOUT, (*coord).into());

                        (
                            *coord,
                            RenderUnit {
                                instance: InstanceData::default().add_translation(vec3(
                                    p.x as Float,
                                    p.y as Float,
                                    FAR as Float,
                                )),
                                tile: *id,
                                model,
                            },
                        )
                    })
            })
            .collect();

        let info = Arc::new(instances);

        if self.render_cache.len() > 16 {
            self.render_cache.clear();
        }

        self.render_cache.insert((range, center), info.clone());

        info
    }
}
