use std::mem;
use std::sync::Arc;

use ractor::concurrency::MpscSender;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};

use automancy_defs::cg::{Float, Matrix4};
use automancy_defs::cgmath::vec3;
use automancy_defs::coord::{TileCoord, TileHex, TileUnit};
use automancy_defs::flexstr::SharedStr;
use automancy_defs::hashbrown::HashMap;
use automancy_defs::hexagon_tiles::layout::hex_to_pixel;
use automancy_defs::hexagon_tiles::traits::HexDirection;
use automancy_defs::id::Id;
use automancy_defs::log;
use automancy_defs::rendering::{InstanceData, HEX_GRID_LAYOUT};
use automancy_resources::data::item::item_match;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::script::Script;
use automancy_resources::ResourceManager;

use crate::game::map::{Map, MapInfo, TileEntities};
use crate::game::state::GameMsg::*;
use crate::game::tile::entity::{TileEntity, TileEntityMsg, TileModifier};
use crate::game::tile::ticking::{tick, TickUnit};
use crate::render::camera::FAR;

const UNDO_CACHE_SIZE: usize = 16;

#[derive(Debug, Clone)]
pub struct RenderUnit {
    pub instance: InstanceData,
    pub tile: Id,
    pub model: Id,
}

pub type RenderInfo = HashMap<TileCoord, RenderUnit>;

#[derive(Debug)]
pub struct GameState {
    /// a count of all the ticks that have happened
    pub(crate) tick_count: TickUnit,

    /// the automancy_resources manager
    pub(crate) resource_man: Arc<ResourceManager>,

    /// the tile entities
    pub(crate) tile_entities: TileEntities,

    /// the map
    map: Map,

    /// render cache
    pub render_cache: HashMap<(TileUnit, TileCoord), Arc<RenderInfo>>,
    /// is the game stopped
    stopped: bool,
    /// the last time the tiles are updated
    last_tiles_update_time: TickUnit,
    /// what to do to undo the last 16 user events
    undo_steps: [Option<Vec<GameMsg>>; UNDO_CACHE_SIZE],
    /// the current index of the undo steps
    undo_steps_index: usize,
}

/// Represents a message the game receives
#[derive(Debug)]
pub enum GameMsg {
    /// tick the tile once
    Tick,
    /// signals that the tiles have been updated in a meaningful way
    SignalTilesUpdated,
    /// get the last tile update time
    LastTilesUpdate(RpcReplyPort<TickUnit>),
    /// get rendering information
    RenderInfoRequest {
        center: TileCoord,
        range: TileUnit,
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
    /// get the tile at the given position
    GetTile(TileCoord, RpcReplyPort<Option<(Id, TileModifier)>>),
    /// get the tile entity at the given position
    GetTileEntity(TileCoord, RpcReplyPort<Option<ActorRef<TileEntityMsg>>>),
    /// get the tile entity at the given position
    GetTileEntityMulti(TileCoord, MpscSender<Option<ActorRef<TileEntityMsg>>>),
    /// send a message to a tile entity
    ForwardMsgToTile(TileCoord, TileEntityMsg),
    /// checks for the adjacent tiles against the script
    CheckAdjacent {
        script: Script,
        coord: TileCoord,
        self_coord: TileCoord,
    },
    TakeTileEntities(RpcReplyPort<TileEntities>),
    /// get the map
    TakeMap(RpcReplyPort<Map>),
    SaveMap(Arc<ResourceManager>, RpcReplyPort<()>),
    /// load a map
    LoadMap(Arc<ResourceManager>, String),
    GetMapInfo(RpcReplyPort<MapInfo>),
    GetUnloadedMapInfo(String, Arc<ResourceManager>, RpcReplyPort<MapInfo>),
    GetDataMap(RpcReplyPort<DataMap>),
    GetDataValue(Id, RpcReplyPort<Option<Data>>),
    SetData(Id, Data),
    RemoveData(Id),
    StopTicking,
}

#[derive(Debug, Copy, Clone)]
pub enum PlaceTileResponse {
    Placed,
    Removed,
    Ignored,
}

pub struct Game;

#[async_trait::async_trait]
impl Actor for Game {
    type Msg = GameMsg;
    type State = GameState;
    type Arguments = (Arc<ResourceManager>, SharedStr);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(Self::State::new(args.0, args.1))
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
                state.tile_entities.values().for_each(|tile_entity| {
                    tile_entity.stop(Some("Loading new map".to_string()));
                });

                let (map, tile_entities) = Map::load(&myself, &resource_man, name.clone()).await;

                state.map = map;
                state.tile_entities = tile_entities;
                state.render_cache.clear();
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
                let map_name = state.map.map_name.clone();
                let tiles = state.map.tiles.len();
                let data = state.map.data.0.len();
                let save_time = state.map.save_time;
                reply
                    .send(MapInfo {
                        map_name,
                        tiles,
                        data,
                        save_time,
                    })
                    .unwrap();
                return Ok(());
            }
            GetUnloadedMapInfo(map, resource_man, reply) => {
                let map = Map::load(&myself, &resource_man, map).await.0;
                reply
                    .send(MapInfo {
                        map_name: map.map_name.clone(),
                        tiles: map.tiles.len(),
                        data: map.data.0.len(),
                        save_time: map.save_time,
                    })
                    .unwrap();
                drop(map);
                return Ok(());
            }
            GetDataMap(reply) => {
                reply.send(mem::take(&mut state.map.data)).unwrap();

                return Ok(());
            }
            GetDataValue(key, reply) => {
                reply.send(state.map.data.get(&key).cloned()).unwrap();

                return Ok(());
            }
            LastTilesUpdate(reply) => {
                reply.send(state.last_tiles_update_time).unwrap();

                return Ok(());
            }
            rest => {
                if state.stopped {
                    return Ok(());
                }

                match rest {
                    SignalTilesUpdated => {
                        state.last_tiles_update_time = state.tick_count;
                    }
                    Tick => {
                        tick(state);
                    }
                    RenderInfoRequest {
                        center,
                        range,
                        reply,
                    } => {
                        let render_info = render_info(state, center, range);

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

                        let old_tile = if id == state.resource_man.registry.none {
                            if !state.map.tiles.contains_key(&coord) {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored).unwrap();
                                }

                                return Ok(());
                            }

                            if let Some(reply) = reply {
                                reply.send(PlaceTileResponse::Removed).unwrap();
                            }

                            stop_tile(state, coord)
                        } else {
                            if let Some(reply) = reply {
                                reply.send(PlaceTileResponse::Placed).unwrap();
                            }

                            insert_new_tile(&myself, state, coord, id, tile_modifier).await
                        };

                        if record {
                            let (id, tile_modifier) =
                                old_tile.unwrap_or((state.resource_man.registry.none, 0));

                            add_undo_step(
                                state,
                                vec![PlaceTile {
                                    coord,
                                    id,
                                    tile_modifier,
                                    record: false,
                                    reply: None,
                                }],
                            );
                        }

                        state.last_tiles_update_time = state.tick_count;

                        state.render_cache.clear();
                    }
                    GetTile(coord, reply) => {
                        reply.send(state.map.tiles.get(&coord).cloned()).unwrap();
                    }
                    GetTileEntity(coord, reply) => {
                        reply
                            .send(state.tile_entities.get(&coord).cloned())
                            .unwrap();
                    }
                    GetTileEntityMulti(coord, reply) => {
                        reply
                            .send(state.tile_entities.get(&coord).cloned())
                            .await
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
                                    if item_match(&state.resource_man.registry, *id, adjacent) {
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
                        undo_once(&myself, state);
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
fn stop_tile(state: &mut GameState, coord: TileCoord) -> Option<(Id, TileModifier)> {
    if let Some(tile_entity) = state.tile_entities.get(&coord) {
        tile_entity.stop(Some("Removed from game".to_string()));
    }

    state.tile_entities.remove(&coord);
    state.map.tiles.remove(&coord)
}

/// Undoes the last undo-able action with step stored in the undo steps array
fn undo_once(game: &ActorRef<GameMsg>, state: &mut GameState) {
    if state.undo_steps.iter().all(Option::is_none) {
        return;
    }

    if state.undo_steps_index == 0 {
        state.undo_steps_index = UNDO_CACHE_SIZE - 1;
    } else {
        state.undo_steps_index -= 1;
    }

    if let Some(step) = mem::take(state.undo_steps.get_mut(state.undo_steps_index).unwrap()) {
        for msg in step {
            game.send_message(msg).unwrap();
        }
    }
}

/// Creates a new tile of given type at the given position, and with an initial state.
pub(crate) async fn new_tile(
    game: &ActorRef<GameMsg>,
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
            game: game.clone(),
        },
        (),
        game.get_cell(),
    )
    .await
    .unwrap();

    actor
}

/// Makes a new tile and add it into both the map and the game
async fn insert_new_tile(
    game: &ActorRef<GameMsg>,
    state: &mut GameState,
    coord: TileCoord,
    id: Id,
    tile_modifier: TileModifier,
) -> Option<(Id, TileModifier)> {
    let old = stop_tile(state, coord);

    let tile_entity = new_tile(game, coord, id, tile_modifier).await;

    state.tile_entities.insert(coord, tile_entity);
    state.map.tiles.insert(coord, (id, tile_modifier));

    old
}

/// Adds one vector of undo steps to the undo steps array
fn add_undo_step(state: &mut GameState, step: Vec<GameMsg>) {
    state.undo_steps[state.undo_steps_index] = Some(step);

    if state.undo_steps_index == UNDO_CACHE_SIZE - 1 {
        state.undo_steps_index = 0;
    } else {
        state.undo_steps_index += 1;
    }
}

fn render_info(state: &mut GameState, center: TileCoord, range: TileUnit) -> Arc<RenderInfo> {
    if let Some(info) = state.render_cache.get(&(range, center)) {
        return info.clone();
    }

    let instances: RenderInfo = state
        .map
        .tiles
        .iter()
        .filter(|(coord, _)| center.distance(**coord) <= range)
        .flat_map(|(coord, (id, tile_modifier))| {
            state
                .resource_man
                .registry
                .tile(*id)
                .and_then(|r| r.models.get(*tile_modifier as usize).cloned())
                .map(|model| {
                    let p = hex_to_pixel(HEX_GRID_LAYOUT, (*coord).into());

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

    state.render_cache.insert((range, center), info.clone());

    info
}

impl GameState {
    /// Creates a new game messaging/map system.
    pub fn new(resource_man: Arc<ResourceManager>, map_name: SharedStr) -> Self {
        Self {
            tick_count: 0,

            resource_man,

            tile_entities: Default::default(),
            map: Map::new_empty(map_name.to_string()),

            render_cache: Default::default(),
            stopped: false,
            last_tiles_update_time: 0,
            undo_steps: Default::default(),
            undo_steps_index: 0,
        }
    }
}
