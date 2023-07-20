use std::collections::VecDeque;
use std::mem;
use std::ops::Div;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use arraydeque::{ArrayDeque, Wrapping};
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};
use rayon::prelude::*;

use automancy_defs::cgmath::vec3;
use automancy_defs::coord::{TileCoord, TileHex, TileUnit};
use automancy_defs::hashbrown::HashMap;
use automancy_defs::hexagon_tiles::traits::HexDirection;
use automancy_defs::id::Id;
use automancy_defs::math::{Float, Matrix4, FAR};
use automancy_defs::rendering::InstanceData;
use automancy_defs::{log, math};
use automancy_resources::data::item::item_match;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::script::Script;
use automancy_resources::ResourceManager;

use crate::game::GameMsg::*;
use crate::map::{Map, MapInfo, TileEntities};
use crate::tile_entity::{TileEntity, TileEntityMsg, TileModifier};

/// Miscellaneous updates per second -- e.g. Camera Position.
pub const UPS: u64 = 60;
pub const UPDATE_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / UPS);

/// Game ticks per second
pub const TPS: u64 = 30;
pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
pub const MAX_ALLOWED_TICK_INTERVAL: Duration = TICK_INTERVAL.saturating_mul(5);

pub const TRANSACTION_ANIMATION_SPEED: Duration = Duration::from_nanos(666_666_666);
pub const TAKE_ITEM_ANIMATION_SPEED: Duration = Duration::from_nanos(200_000_000);

pub type TickUnit = u16;

#[derive(Debug, Clone, Copy)]
pub struct TransactionRecord {
    pub stack: ItemStack,
    pub source_id: Id,
    pub id: Id,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderUnit {
    pub instance: InstanceData,
    pub tile: Id,
    pub model: Id,
}

pub type RenderInfo = HashMap<TileCoord, RenderUnit>;
pub type TransactionRecords =
    HashMap<(TileCoord, TileCoord), VecDeque<(Instant, TransactionRecord)>>;

#[derive(Debug)]
pub struct GameState {
    /// the tile entities
    tile_entities: TileEntities,
    /// the map
    map: Map,

    /// a count of all the ticks that have happened
    tick_count: TickUnit,
    /// render cache
    render_cache: HashMap<(TileCoord, (TileUnit, TileUnit)), Arc<RenderInfo>>,
    /// is the game stopped
    stopped: bool,
    /// what to do to undo the last UNDO_CACHE_SIZE user events
    undo_steps: ArrayDeque<Vec<GameMsg>, 16, Wrapping>,
    /// records transactions to be drawn
    transaction_records: Arc<RwLock<TransactionRecords>>,
}

/// Represents a message the game receives
#[derive(Debug)]
pub enum GameMsg {
    /// tick the tile once
    Tick,
    /// get rendering information
    RenderInfoRequest {
        center: TileCoord,
        culling_range: (TileUnit, TileUnit),
        reply: RpcReplyPort<Arc<RenderInfo>>,
    },
    /// place a tile at the given position
    PlaceTile {
        coord: TileCoord,
        id: Id,
        tile_modifier: TileModifier,
        record: bool,
        reply: Option<RpcReplyPort<PlaceTileResponse>>,
    },
    Undo,
    /// send a message to a tile entity
    ForwardMsgToTile(TileCoord, TileEntityMsg),
    /// checks for the adjacent tiles against the script
    CheckAdjacent {
        script: Script,
        coord: TileCoord,
        self_coord: TileCoord,
    },
    /// get the tile at the given position
    GetTile(TileCoord, RpcReplyPort<Option<(Id, TileModifier)>>),
    /// get the tile entity at the given position
    GetTileEntity(TileCoord, RpcReplyPort<Option<ActorRef<TileEntityMsg>>>),
    /// get all the tile entities
    GetTileEntities {
        center: TileCoord,
        culling_range: (TileUnit, TileUnit),
        reply: RpcReplyPort<TileEntities>,
    },
    /// take the tile entities
    TakeTileEntities(RpcReplyPort<TileEntities>),
    /// take the map
    TakeMap(RpcReplyPort<Map>),
    SaveMap(Arc<ResourceManager>, RpcReplyPort<()>),
    MoveTiles(Vec<TileCoord>, TileCoord, bool),
    /// load a map
    LoadMap(Arc<ResourceManager>, String),
    GetMapInfo(RpcReplyPort<(MapInfo, String)>),
    TakeDataMap(RpcReplyPort<DataMap>),
    GetDataValue(Id, RpcReplyPort<Option<Data>>),
    SetDataMap(DataMap),
    SetData(Id, Data),
    RemoveData(Id),
    StopTicking,
    GetRecordedTransactions(RpcReplyPort<Arc<RwLock<TransactionRecords>>>),
    RecordTransaction(ItemStack, TileCoord, TileCoord),
}

#[derive(Debug, Copy, Clone)]
pub enum PlaceTileResponse {
    Placed,
    Removed,
    Ignored,
}

pub struct Game {
    pub resource_man: Arc<ResourceManager>,
}

impl Game {
    fn render_info(
        &self,
        state: &mut GameState,
        center: TileCoord,
        culling_range: (TileUnit, TileUnit),
    ) -> Arc<RenderInfo> {
        if let Some(info) = state.render_cache.get(&(center, culling_range)) {
            return info.clone();
        }

        let instances: RenderInfo = state
            .map
            .tiles
            .iter()
            .filter(|(coord, _)| math::is_in_culling_range(center, **coord, culling_range))
            .flat_map(|(coord, (id, tile_modifier))| {
                self.resource_man
                    .registry
                    .tile(*id)
                    .and_then(|r| r.models.get(*tile_modifier as usize).cloned())
                    .map(|id| self.resource_man.get_model(id))
                    .map(|model| {
                        let p = math::hex_to_pixel((*coord).into());

                        (
                            *coord,
                            RenderUnit {
                                instance: InstanceData::default().with_model_matrix(
                                    Matrix4::from_translation(vec3(
                                        p.x as Float,
                                        p.y as Float,
                                        FAR as Float,
                                    )),
                                ),
                                tile: *id,
                                model,
                            },
                        )
                    })
            })
            .collect();

        let info = Arc::new(instances);

        if state.render_cache.len() > 16 {
            state.render_cache.clear();
        }

        state
            .render_cache
            .insert((center, culling_range), info.clone());

        info
    }
}

#[async_trait::async_trait]
impl Actor for Game {
    type Msg = GameMsg;
    type State = GameState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(Self::State::default())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            TakeTileEntities(reply) => {
                reply.send(mem::take(&mut state.tile_entities)).unwrap();

                return Ok(());
            }
            TakeMap(reply) => {
                let map_name = state.map.map_name.clone();

                reply
                    .send(mem::replace(&mut state.map, Map::new_empty(map_name)))
                    .unwrap();

                return Ok(());
            }
            LoadMap(resource_man, name) => {
                for tile_entity in state.tile_entities.values() {
                    tile_entity.stop(Some("Loading new map".to_string()));
                }

                let (map, tile_entities) = Map::load(myself.clone(), resource_man, &name).await;

                state.map = map;
                state.tile_entities = tile_entities;
                state.render_cache.clear();
                state.transaction_records.write().unwrap().clear();
                state.undo_steps.clear();

                log::info!("Successfully loaded map {name}!");
                return Ok(());
            }
            SaveMap(resource_man, reply) => {
                state
                    .map
                    .save(&resource_man.interner, &state.tile_entities)
                    .await;
                log::info!("Saved map {}", state.map.map_name.clone());
                reply.send(()).unwrap();
            }
            GetMapInfo(reply) => {
                let tile_count = state.map.tiles.len() as u64;
                let save_time = state.map.save_time;

                reply
                    .send((
                        MapInfo {
                            tile_count,
                            save_time,
                        },
                        state.map.map_name.clone(),
                    ))
                    .unwrap();

                return Ok(());
            }
            TakeDataMap(reply) => {
                reply.send(mem::take(&mut state.map.data)).unwrap();

                return Ok(());
            }
            SetDataMap(data) => {
                state.map.data = data;
            }
            GetDataValue(key, reply) => {
                reply.send(state.map.data.get(&key).cloned()).unwrap();

                return Ok(());
            }
            rest => {
                if state.stopped {
                    return Ok(());
                }

                match rest {
                    Tick => {
                        tick(state);
                    }
                    RenderInfoRequest {
                        center,
                        culling_range,
                        reply,
                    } => {
                        let render_info = self.render_info(state, center, culling_range);

                        reply.send(render_info).unwrap();
                    }
                    PlaceTile {
                        coord,
                        id,
                        tile_modifier,
                        record,
                        reply,
                    } => {
                        if let Some((old_id, old_tile_modifier)) = state.map.tiles.get(&coord) {
                            if *old_tile_modifier == tile_modifier && *old_id == id {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored).unwrap();
                                }

                                return Ok(());
                            }
                        }

                        let old_tile = if id == self.resource_man.registry.none {
                            if !state.map.tiles.contains_key(&coord) {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored).unwrap();
                                }

                                return Ok(());
                            }

                            if let Some(reply) = reply {
                                reply.send(PlaceTileResponse::Removed).unwrap();
                            }

                            remove_tile(state, coord)
                        } else {
                            if let Some(reply) = reply {
                                reply.send(PlaceTileResponse::Placed).unwrap();
                            }

                            insert_new_tile(
                                self.resource_man.clone(),
                                myself.clone(),
                                state,
                                coord,
                                id,
                                tile_modifier,
                            )
                            .await
                        };

                        if record {
                            let (id, tile_modifier) =
                                old_tile.unwrap_or((self.resource_man.registry.none, 0));

                            state.undo_steps.push_back(vec![PlaceTile {
                                coord,
                                id,
                                tile_modifier,
                                record: false,
                                reply: None,
                            }]);
                        }
                    }
                    GetTile(coord, reply) => {
                        reply.send(state.map.tiles.get(&coord).cloned()).unwrap();
                    }
                    GetTileEntity(coord, reply) => {
                        reply
                            .send(state.tile_entities.get(&coord).cloned())
                            .unwrap();
                    }
                    GetTileEntities {
                        center,
                        culling_range,
                        reply,
                    } => {
                        reply
                            .send(
                                state
                                    .tile_entities
                                    .iter()
                                    .filter(|(coord, _)| {
                                        math::is_in_culling_range(center, **coord, culling_range)
                                    })
                                    .map(|(coord, entity)| (*coord, entity.clone()))
                                    .collect(),
                            )
                            .unwrap();
                    }
                    ForwardMsgToTile(coord, msg) => {
                        if let Some(tile_entity) = state.tile_entities.get(&coord) {
                            tile_entity.send_message(msg).unwrap();
                        }
                    }
                    SetData(key, value) => {
                        state.map.data.0.insert(key, value);
                    }
                    RemoveData(key) => {
                        state.map.data.remove(&key);
                    }
                    CheckAdjacent {
                        script,
                        coord,
                        self_coord,
                    } => {
                        if let Some(adjacent) = script.adjacent {
                            let mut fulfilled = false;

                            for neighbor in TileHex::NEIGHBORS.iter().map(|v| coord + (*v).into()) {
                                if let Some((id, _)) = state.map.tiles.get(&neighbor) {
                                    if item_match(&self.resource_man, *id, adjacent) {
                                        fulfilled = true;
                                        break;
                                    }
                                }
                            }

                            if let Some(entity) = state.tile_entities.get(&self_coord) {
                                entity
                                    .send_message(TileEntityMsg::AdjacentState { fulfilled })
                                    .unwrap();
                            }
                        } else if let Some(entity) = state.tile_entities.get(&self_coord) {
                            entity
                                .send_message(TileEntityMsg::AdjacentState { fulfilled: true })
                                .unwrap();
                        }
                    }
                    StopTicking => {
                        state.stopped = true;
                    }
                    Undo => {
                        if let Some(step) = state.undo_steps.pop_back() {
                            for msg in step {
                                myself.send_message(msg).unwrap();
                            }
                        }
                    }
                    GetRecordedTransactions(reply) => {
                        let now = Instant::now();

                        let mut to_remove = HashMap::new();

                        for (coord, deque) in state.transaction_records.read().unwrap().iter() {
                            to_remove.insert(
                                *coord,
                                deque
                                    .iter()
                                    .take_while(|(instant, _)| {
                                        now.duration_since(*instant) >= TRANSACTION_ANIMATION_SPEED
                                    })
                                    .count(),
                            );
                        }

                        let mut record = state.transaction_records.write().unwrap();
                        for (coord, v) in to_remove {
                            for _ in 0..v {
                                record.get_mut(&coord).unwrap().pop_front();
                            }
                        }

                        reply.send(state.transaction_records.clone()).unwrap();
                    }
                    RecordTransaction(stack, source_coord, coord) => {
                        if let Some((instant, _)) = state
                            .transaction_records
                            .read()
                            .unwrap()
                            .get(&(source_coord, coord))
                            .and_then(|v| v.back())
                        {
                            if Instant::now().duration_since(*instant)
                                < TRANSACTION_ANIMATION_SPEED.div(4)
                            {
                                return Ok(());
                            }
                        }

                        if let Some(((source_id, _), (id, _))) = state
                            .map
                            .tiles
                            .get(&source_coord)
                            .cloned()
                            .zip(state.map.tiles.get(&coord).cloned())
                        {
                            state
                                .transaction_records
                                .write()
                                .unwrap()
                                .entry((source_coord, coord))
                                .or_insert_with(Default::default)
                                .push_back((
                                    Instant::now(),
                                    TransactionRecord {
                                        stack,
                                        source_id,
                                        id,
                                    },
                                ));
                        }
                    }
                    MoveTiles(tiles, direction, record) => {
                        let mut undo = vec![];

                        tiles
                            .into_iter()
                            .flat_map(|coord| {
                                Some(coord).zip(
                                    state
                                        .map
                                        .tiles
                                        .remove(&coord)
                                        .zip(state.tile_entities.remove(&coord)),
                                )
                            })
                            .collect::<Vec<_>>()
                            .into_iter()
                            .for_each(|(coord, ((id, modifier), tile_entity))| {
                                let new_coord = coord + direction;

                                state.map.tiles.insert(new_coord, (id, modifier));
                                state.tile_entities.insert(new_coord, tile_entity);

                                undo.push(new_coord);
                            });

                        if record {
                            state
                                .undo_steps
                                .push_back(vec![MoveTiles(undo, -direction, false)]);
                        }

                        state.render_cache.clear();
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: SupervisionEvent,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorPanicked(_dead_actor, panic_msg) => {
                panic!(
                    "game: panicked because tile entity panicked with '{}'",
                    panic_msg
                );
            }
            SupervisionEvent::ActorTerminated(dead_actor, _tile_state, reason) => {
                log::debug!("game: tile entity {dead_actor:?} has been removed. reason (if any): {reason:?}")
            }
            other => {
                log::debug!("game: supervision event: {other}")
            }
        }
        Ok(())
    }
}

/// Stops a tile and removes it from the game
fn remove_tile(state: &mut GameState, coord: TileCoord) -> Option<(Id, TileModifier)> {
    state.render_cache.clear();

    if let Some(tile_entity) = state.tile_entities.get(&coord) {
        tile_entity.stop(Some("Removed from game".to_string()));
    }

    state.tile_entities.remove(&coord);
    state.map.tiles.remove(&coord)
}

/// Creates a new tile of given type at the given position, and with an initial state.
pub async fn new_tile(
    resource_man: Arc<ResourceManager>,
    game: ActorRef<GameMsg>,
    coord: TileCoord,
    id: Id,
    tile_modifier: TileModifier,
) -> ActorRef<TileEntityMsg> {
    let (actor, _handle) = Actor::spawn_linked(
        None,
        TileEntity {
            id,
            coord,
            tile_modifier,
            resource_man,
        },
        (game.clone(),),
        game.get_cell(),
    )
    .await
    .unwrap();

    actor
}

/// Makes a new tile and add it into both the map and the game
async fn insert_new_tile(
    resource_man: Arc<ResourceManager>,
    game: ActorRef<GameMsg>,
    state: &mut GameState,
    coord: TileCoord,
    id: Id,
    tile_modifier: TileModifier,
) -> Option<(Id, TileModifier)> {
    let old = remove_tile(state, coord);

    let tile_entity = new_tile(resource_man, game, coord, id, tile_modifier).await;

    state.tile_entities.insert(coord, tile_entity);
    state.map.tiles.insert(coord, (id, tile_modifier));

    old
}

fn inner_tick(state: &mut GameState) {
    state.tile_entities.par_iter().for_each(|(_, tile_entity)| {
        if let Err(e) = tile_entity.send_message(TileEntityMsg::Tick {
            tick_count: state.tick_count,
        }) {
            log::error!("{e:?}");
        }
    });

    state.tick_count = state.tick_count.wrapping_add(1);
}

/// Runs the game for one tick, logging if the tick is too long.
pub fn tick(state: &mut GameState) {
    let start = Instant::now();
    inner_tick(state);
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

impl Default for GameState {
    fn default() -> Self {
        Self {
            map: Map::new_empty("".to_string()),
            tile_entities: Default::default(),

            tick_count: 0,
            render_cache: Default::default(),
            stopped: false,
            undo_steps: Default::default(),
            transaction_records: Arc::new(Default::default()),
        }
    }
}
