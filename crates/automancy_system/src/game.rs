use crate::map::{GameMap, MapInfo, TileEntities};
use crate::tile_entity::{TileEntity, TileEntityMsg};
use crate::{game::GameSystemMessage::*, map::LoadMapOption};
use crate::{tile_entity::TileEntityError, util::actor::multi_call_iter};
use arraydeque::{ArrayDeque, Wrapping};
use automancy_defs::id::{Id, ModelId, RenderTagId};
use automancy_defs::{
    coord::{TileBounds, TileCoord},
    id::TileId,
};
use automancy_resources::types::function::OnFailAction;
use automancy_resources::ResourceManager;
use automancy_resources::{
    data::{Data, DataMap},
    rhai_render::RenderCommand,
};
use hashbrown::HashMap;
use ractor::rpc::CallResult;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};
use std::time::{Duration, Instant};
use std::{mem, sync::Arc};
use tokio::sync::Mutex;

/// Game ticks per second
pub const TPS: u64 = 60;
pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
pub const MAX_ALLOWED_TICK_INTERVAL: Duration = TICK_INTERVAL.saturating_mul(5);

pub const TRANSACTION_ANIMATION_SPEED: Duration = Duration::from_nanos(800_000_000);
pub const TRANSACTION_MIN_INTERVAL: Duration = Duration::from_nanos(250_000_000);
pub const TAKE_ITEM_ANIMATION_SPEED: Duration = Duration::from_nanos(300_000_000);

const UNDO_CACHE_SIZE: usize = 256;

pub type TickUnit = u16;

pub type FlatTiles = Vec<(TileCoord, TileId, Option<DataMap>)>;

#[derive(Debug, Default)]
pub struct GameSystemState {
    /// a count of all the ticks that have happened
    tick_count: TickUnit,
    /// is the game stopped
    stopped: bool,

    /// the tile entities
    tile_entities: TileEntities,
    /// the map
    map: Option<GameMap>,

    /// what to do to undo the last UNDO_CACHE_SIZE user events
    undo_steps: ArrayDeque<Vec<GameSystemMessage>, UNDO_CACHE_SIZE, Wrapping>,

    cleanup_render_commands: HashMap<TileCoord, Vec<RenderCommand>>,
    last_culling_range: TileBounds,
}

pub static COULD_NOT_LOAD_ANYTHING: &str = "??? main menu is corrupted and couldn't be emptied!";

fn track_none(resource_man: &ResourceManager, coord: TileCoord) -> [RenderCommand; 2] {
    [
        RenderCommand::Track {
            tag: RenderTagId(resource_man.registry.data_ids.none_tile_render_tag),
            model: ModelId(resource_man.registry.model_ids.tile_none),
        },
        RenderCommand::Transform {
            tag: RenderTagId(resource_man.registry.data_ids.none_tile_render_tag),
            model: ModelId(resource_man.registry.model_ids.tile_none),
            model_matrix: coord.as_translation(),
        },
    ]
}

fn untrack_none(resource_man: &ResourceManager) -> [RenderCommand; 1] {
    [RenderCommand::Untrack {
        tag: RenderTagId(resource_man.registry.data_ids.none_tile_render_tag),
        model: ModelId(resource_man.registry.model_ids.tile_none),
    }]
}

fn fill_map_with_none(
    resource_man: &ResourceManager,
    culling_range: TileBounds,
    last_culling_range: TileBounds,
    commands: &mut HashMap<TileCoord, Vec<RenderCommand>>,
) {
    if culling_range == last_culling_range {
        return;
    }

    for coord in culling_range.into_iter() {
        if !commands.contains_key(&coord) && !last_culling_range.contains(coord) {
            commands.insert(coord, track_none(resource_man, coord).to_vec());
        }
    }

    for coord in last_culling_range.into_iter() {
        if !commands.contains_key(&coord) && !culling_range.contains(coord) {
            commands.insert(coord, untrack_none(resource_man).to_vec());
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PlaceTileResponse {
    Placed,
    Removed,
    Ignored,
}

/// Represents a message the game receives
#[derive(Debug)]
pub enum GameSystemMessage {
    /// tick the tiles once
    Tick,
    StopTicking,

    /// load a map
    LoadMap(LoadMapOption, RpcReplyPort<bool>),
    /// save the map
    SaveMap(RpcReplyPort<()>),
    GetMapInfoAndName(RpcReplyPort<Option<(Arc<Mutex<MapInfo>>, LoadMapOption)>>),

    /// send a message to a tile entity
    ForwardMsgToTile {
        source: TileCoord,
        to: TileCoord,
        msg: TileEntityMsg,
        on_fail: OnFailAction,
    },

    /// place a tile at the given position
    PlaceTile {
        coord: TileCoord,
        id: TileId,
        data: Option<DataMap>,
        record: bool,
        reply: Option<RpcReplyPort<PlaceTileResponse>>,
    },
    PlaceTiles {
        tiles: FlatTiles,
        reply: Option<RpcReplyPort<FlatTiles>>,
        place_over: bool,
        record: bool,
    },
    MoveTiles(Vec<TileCoord>, TileCoord, bool),

    Undo,

    /// get the tile at the given position
    GetTile(TileCoord, RpcReplyPort<Option<TileId>>),
    /// get the tile entity at the given position
    GetTileEntity(TileCoord, RpcReplyPort<Option<ActorRef<TileEntityMsg>>>),
    GetTiles(Vec<TileCoord>, RpcReplyPort<FlatTiles>),
    /// get all the tiles' render commands
    GetAllRenderCommands {
        culling_range: TileBounds,
        reply: RpcReplyPort<[HashMap<TileCoord, Vec<RenderCommand>>; 2]>,
    },
}

pub struct GameSystem {
    pub resource_man: Arc<ResourceManager>,
}

#[async_trait::async_trait]
impl Actor for GameSystem {
    type Msg = GameSystemMessage;
    type State = GameSystemState;
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
            LoadMap(opt, reply) => {
                let last_culling_range = state.last_culling_range;
                state.last_culling_range = TileBounds::Empty;

                let commands = multi_call_iter(
                    &state.tile_entities,
                    |reply, coord| TileEntityMsg::CollectRenderCommands {
                        reply,
                        loading: false,
                        unloading: last_culling_range.contains(coord),
                    },
                    None,
                )
                .await
                .map(|commands| {
                    commands
                        .into_iter()
                        .flat_map(|(k, v)| Some(k).zip(v))
                        .collect()
                });

                match commands {
                    Ok(mut commands) => {
                        fill_map_with_none(
                            &self.resource_man,
                            TileBounds::Empty,
                            last_culling_range,
                            &mut commands,
                        );
                        state.cleanup_render_commands = commands;
                    }
                    Err(err) => {
                        log::error!("Map render cleanup failed to collect commands! Error: {err:?}")
                    }
                }

                for tile_entity in mem::take(&mut state.tile_entities).into_values() {
                    tile_entity
                        .stop_and_wait(Some("Loading new map".to_string()), None)
                        .await
                        .unwrap();
                }

                state.map = None;
                state.undo_steps.clear();

                let (map, tile_entities) =
                    match GameMap::load(myself.clone(), self.resource_man.clone(), &opt).await {
                        Ok(v) => v,
                        Err(abort) => {
                            if abort {
                                reply.send(false)?;
                                return Ok(());
                            } else {
                                (GameMap::new_empty(opt.clone()), HashMap::new())
                            }
                        }
                    };

                state.map = Some(map);
                state.tile_entities = tile_entities;

                log::info!("Successfully loaded map {opt}!");
                reply.send(true)?;
            }
            SaveMap(reply) => {
                if let Some(map) = &state.map {
                    map.save(&self.resource_man.interner, &state.tile_entities)
                        .await?;
                }
                reply.send(())?;
            }
            GetMapInfoAndName(reply) => {
                if let Some(map) = &state.map {
                    reply.send(Some((map.info.clone(), map.opt.clone())))?;
                } else {
                    reply.send(None)?;
                }
            }

            Tick => {
                tick(state);
            }
            StopTicking => {
                state.stopped = true;
            }

            rest => {
                if state.stopped {
                    return Ok(());
                }

                let Some(map) = state.map.as_mut() else {
                    return Ok(());
                };

                match rest {
                    GetAllRenderCommands {
                        culling_range,
                        reply,
                    } => {
                        let last_culling_range = state.last_culling_range;
                        state.last_culling_range = culling_range;

                        let commands = multi_call_iter(
                            &state.tile_entities,
                            |reply, coord| {
                                let loading = culling_range.contains(coord)
                                    && !last_culling_range.contains(coord);
                                let unloading = last_culling_range.contains(coord)
                                    && !culling_range.contains(coord);

                                TileEntityMsg::CollectRenderCommands {
                                    reply,
                                    loading,
                                    unloading,
                                }
                            },
                            None,
                        )
                        .await
                        .map(|commands| {
                            commands
                                .into_iter()
                                .flat_map(|(k, v)| Some(k).zip(v))
                                .collect()
                        });

                        match commands {
                            Ok(mut commands) => {
                                fill_map_with_none(
                                    &self.resource_man,
                                    culling_range,
                                    last_culling_range,
                                    &mut commands,
                                );

                                reply.send([
                                    mem::take(&mut state.cleanup_render_commands),
                                    commands,
                                ])?;
                            }
                            Err(err) => {
                                log::error!("Could not collect render commands! Error: {err:?}")
                            }
                        }
                    }
                    PlaceTile {
                        coord,
                        id,
                        data,
                        record,
                        reply,
                    } => {
                        if let Some(old_id) = map.tiles.get(&coord) {
                            if *old_id == id {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored)?;
                                }

                                return Ok(());
                            }
                        }

                        if id == TileId(self.resource_man.registry.none)
                            && !map.tiles.contains_key(&coord)
                        {
                            if let Some(reply) = reply {
                                reply.send(PlaceTileResponse::Ignored)?;
                            }

                            return Ok(());
                        }

                        let old_tile = insert_new_tile(
                            self.resource_man.clone(),
                            myself.clone(),
                            map,
                            &mut state.tile_entities,
                            &mut state.cleanup_render_commands,
                            coord,
                            id,
                            data,
                        )
                        .await;

                        if let Some(reply) = reply {
                            if let (Some(_), ..) = &old_tile {
                                if id == TileId(self.resource_man.registry.none) {
                                    reply.send(PlaceTileResponse::Removed)?;
                                } else {
                                    reply.send(PlaceTileResponse::Placed)?;
                                }
                            } else {
                                reply.send(PlaceTileResponse::Placed)?;
                            }
                        }

                        if let (Some(id), data) = old_tile {
                            if record {
                                state.undo_steps.push_back(vec![PlaceTile {
                                    coord,
                                    id,
                                    record: false,
                                    reply: None,
                                    data,
                                }]);
                            }
                        }
                    }
                    GetTile(coord, reply) => {
                        reply.send(map.tiles.get(&coord).cloned())?;
                    }
                    GetTileEntity(coord, reply) => {
                        reply.send(state.tile_entities.get(&coord).cloned())?;
                    }
                    ForwardMsgToTile {
                        source,
                        to,
                        msg,
                        on_fail,
                    } => {
                        if let Some(tile_entity) = state.tile_entities.get(&to) {
                            tile_entity.send_message(msg)?;
                        } else {
                            match on_fail {
                                OnFailAction::None => {}
                                OnFailAction::RemoveTile => {
                                    remove_tile(
                                        &self.resource_man,
                                        map,
                                        &mut state.tile_entities,
                                        source,
                                    )
                                    .await;
                                }
                                OnFailAction::RemoveAllData => {
                                    if let Some(entity) = state.tile_entities.get(&source) {
                                        entity.send_message(TileEntityMsg::SetData(
                                            DataMap::default(),
                                        ))?;
                                    }
                                }
                                OnFailAction::RemoveData(id) => {
                                    if let Some(entity) = state.tile_entities.get(&source) {
                                        entity.send_message(TileEntityMsg::RemoveData(id))?;
                                    }
                                }
                            }
                        }
                    }
                    Undo => {
                        if let Some(step) = state.undo_steps.pop_back() {
                            for msg in step {
                                myself.send_message(msg)?;
                            }
                        }
                    }
                    GetTiles(coords, reply) => {
                        let mut tiles = vec![];

                        for (id, coord) in coords
                            .into_iter()
                            .flat_map(|coord| map.tiles.get(&coord).zip(Some(coord)))
                        {
                            if let Some(entity) = state.tile_entities.get(&coord) {
                                if let Ok(CallResult::Success(mut data)) =
                                    entity.call(TileEntityMsg::GetData, None).await
                                {
                                    tiles.push((
                                        coord,
                                        *id,
                                        Some(copy_auxiliary_data(&self.resource_man, &mut data)),
                                    ));
                                }
                            } else {
                                tiles.push((coord, *id, None));
                            }
                        }
                        reply.send(tiles)?;
                    }
                    PlaceTiles {
                        tiles,
                        reply,
                        place_over,
                        record,
                    } => {
                        let mut old = vec![];

                        for (coord, id, data) in tiles {
                            if place_over || map.tiles.get(&coord).is_none() {
                                if let (Some(old_id), old_data) = insert_new_tile(
                                    self.resource_man.clone(),
                                    myself.clone(),
                                    map,
                                    &mut state.tile_entities,
                                    &mut state.cleanup_render_commands,
                                    coord,
                                    id,
                                    data,
                                )
                                .await
                                {
                                    if let Some(mut old_data) = old_data {
                                        old.push((
                                            coord,
                                            old_id,
                                            Some(copy_auxiliary_data(
                                                &self.resource_man,
                                                &mut old_data,
                                            )),
                                        ));
                                    } else {
                                        old.push((coord, old_id, None));
                                    }
                                }
                            }
                        }

                        if let Some(reply) = reply {
                            reply.send(old)?;
                        } else if record {
                            state.undo_steps.push_back(vec![PlaceTiles {
                                tiles: old,
                                reply: None,
                                place_over: false,
                                record: false,
                            }]);
                        }
                    }
                    MoveTiles(tiles, direction, record) => {
                        let mut undo = vec![];

                        let mut removed = Vec::new();

                        for coord in tiles {
                            if let Some(old) = remove_tile(
                                &self.resource_man,
                                map,
                                &mut state.tile_entities,
                                coord,
                            )
                            .await
                            {
                                removed.push((coord, old));
                            }
                        }

                        for (coord, (id, data, mut cleanup)) in removed {
                            let new_coord = coord + direction;

                            state
                                .cleanup_render_commands
                                .entry(coord)
                                .or_default()
                                .append(&mut cleanup);

                            insert_new_tile(
                                self.resource_man.clone(),
                                myself.clone(),
                                map,
                                &mut state.tile_entities,
                                &mut state.cleanup_render_commands,
                                new_coord,
                                id,
                                data,
                            )
                            .await;

                            undo.push(new_coord);
                        }

                        if record {
                            state
                                .undo_steps
                                .push_back(vec![MoveTiles(undo, -direction, false)]);
                        }
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
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorFailed(dead_actor, error) => {
                log::error!(
                    "Tile entity {dead_actor:?} panicked, trying to remove. Error: {error}"
                );

                if let Ok(tile_error) = error.downcast::<Box<TileEntityError>>() {
                    match **tile_error {
                        TileEntityError::NonExistent(coord) => {
                            if let Some(map) = state.map.as_mut() {
                                remove_tile(
                                    &self.resource_man,
                                    map,
                                    &mut state.tile_entities,
                                    coord,
                                )
                                .await;
                            }
                        }
                    }
                }
            }
            SupervisionEvent::ActorTerminated(dead_actor, _tile_state, reason) => {
                log::debug!("Tile entity {dead_actor:?} has been removed. Reason: {reason:?}");
            }
            other => {
                log::debug!("Supervision event: {other}")
            }
        }

        Ok(())
    }
}

pub fn try_category(resource_man: &ResourceManager, id: TileId, category_item: impl FnOnce(Id)) {
    if let Some(category) = resource_man
        .registry
        .tiles
        .get(&id)
        .and_then(|tile| tile.category)
    {
        if Data::Bool(false)
            == *resource_man.registry.tiles[&id]
                .data
                .get(resource_man.registry.data_ids.default_tile)
                .unwrap_or(&Data::Bool(false))
        {
            if let Some(item) = resource_man
                .registry
                .categories
                .get(&category)
                .and_then(|v| v.item)
            {
                category_item(item);
            }
        }
    }
}

/// Creates a new tile of given type at the given position, and with an initial state.
pub async fn new_tile(
    resource_man: Arc<ResourceManager>,
    game: ActorRef<GameSystemMessage>,
    coord: TileCoord,
    id: TileId,
) -> ActorRef<TileEntityMsg> {
    let (actor, _handle) = Actor::spawn_linked(
        Some(coord.to_minimal_string()),
        TileEntity {
            id,
            coord,
            resource_man,
        },
        (game.clone(),),
        game.get_cell(),
    )
    .await
    .unwrap();

    actor
}

/// Stops a tile and removes it from the game
async fn remove_tile(
    resource_man: &ResourceManager,
    map: &mut GameMap,
    tile_entities: &mut TileEntities,
    coord: TileCoord,
) -> Option<(TileId, Option<DataMap>, Vec<RenderCommand>)> {
    if let Some((tile, tile_entity)) = map.tiles.remove(&coord).zip(tile_entities.remove(&coord)) {
        {
            let lock = &mut map.info.lock().await;

            try_category(resource_man, tile, |item| {
                if let Data::Inventory(inventory) = lock
                    .data
                    .entry(resource_man.registry.data_ids.player_inventory)
                    .or_insert_with(|| Data::Inventory(Default::default()))
                {
                    inventory.add(item, 1);
                }
            });
        }

        let data = tile_entity
            .call(TileEntityMsg::TakeData, None)
            .await
            .ok()
            .and_then(|v| v.success_or(()).ok());

        let mut commands = tile_entity
            .call(
                |reply| TileEntityMsg::CollectRenderCommands {
                    reply,
                    loading: false,
                    unloading: true,
                },
                None,
            )
            .await
            .unwrap()
            .unwrap()
            .unwrap_or_default();

        commands.extend_from_slice(&track_none(resource_man, coord));

        tile_entity
            .stop_and_wait(Some("Removed from game".to_string()), None)
            .await
            .unwrap();

        Some((tile, data, commands))
    } else {
        None
    }
}

/// Makes a new tile and add it into both the map and the game
async fn insert_new_tile(
    resource_man: Arc<ResourceManager>,
    game: ActorRef<GameSystemMessage>,
    map: &mut GameMap,
    tile_entities: &mut TileEntities,
    cleanup_render_commands: &mut HashMap<TileCoord, Vec<RenderCommand>>,
    coord: TileCoord,
    tile_id: TileId,
    data: Option<DataMap>,
) -> (Option<TileId>, Option<DataMap>) {
    let mut skip = false;

    {
        let lock = &mut map.info.lock().await;

        try_category(&resource_man, tile_id, |item| {
            if let Data::Inventory(inventory) = lock
                .data
                .entry(resource_man.registry.data_ids.player_inventory)
                .or_insert_with(|| Data::Inventory(Default::default()))
            {
                if inventory.get(item) < 1 {
                    skip = true;
                }

                inventory.take(item, 1);
            }
        });
    }

    if skip {
        return (None, None);
    }

    let mut old_id = None;
    let mut old_data = None;

    if let Some((id, data, mut cleanup)) =
        remove_tile(&resource_man, map, tile_entities, coord).await
    {
        cleanup_render_commands
            .entry(coord)
            .or_default()
            .append(&mut cleanup);

        if tile_id == TileId(resource_man.registry.none) {
            return (Some(id), data);
        }

        old_id = Some(id);
        old_data = data;
    }

    let tile_entity = new_tile(resource_man.clone(), game, coord, tile_id).await;

    if let Some(data) = data {
        tile_entity
            .send_message(TileEntityMsg::SetData(data))
            .unwrap();
    }

    cleanup_render_commands
        .entry(coord)
        .or_default()
        .extend_from_slice(&untrack_none(&resource_man));

    let mut new_tile_render = tile_entity
        .call(
            |reply| TileEntityMsg::CollectRenderCommands {
                reply,
                loading: true,
                unloading: false,
            },
            None,
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap_or_default();

    cleanup_render_commands
        .entry(coord)
        .or_default()
        .append(&mut new_tile_render);

    tile_entities.insert(coord, tile_entity);
    map.tiles.insert(coord, tile_id);

    (old_id, old_data)
}

fn inner_tick(state: &mut GameSystemState) {
    state.tile_entities.iter().for_each(|(_, tile_entity)| {
        if let Err(e) = tile_entity.send_message(TileEntityMsg::Tick {
            tick_count: state.tick_count,
        }) {
            log::error!("{e:?}");
        }
    });

    state.tick_count = state.tick_count.wrapping_add(1);
}

/// Runs the game for one tick, logging if the tick is too long.
pub fn tick(state: &mut GameSystemState) {
    let start = Instant::now();
    inner_tick(state);
    let finish = Instant::now();

    let tick_time = finish - start;

    if tick_time >= MAX_ALLOWED_TICK_INTERVAL {
        log::warn!(
            "Tick took longer than allowed maximum! tick_time: {:?}, maximum: {:?}",
            tick_time,
            MAX_ALLOWED_TICK_INTERVAL
        );
    }
}

// TODO replace this with a scripted function
pub fn copy_auxiliary_data(resource_man: &ResourceManager, data: &mut DataMap) -> DataMap {
    let mut copied = DataMap::default();

    if let Some(v) = data.remove(resource_man.registry.data_ids.direction) {
        copied.set(resource_man.registry.data_ids.direction, v);
    }
    if let Some(v) = data.remove(resource_man.registry.data_ids.link) {
        copied.set(resource_man.registry.data_ids.link, v);
    }
    if let Some(v) = data.remove(resource_man.registry.data_ids.script) {
        copied.set(resource_man.registry.data_ids.script, v);
    }
    if let Some(v) = data.remove(resource_man.registry.data_ids.capacity) {
        copied.set(resource_man.registry.data_ids.capacity, v);
    }
    if let Some(v) = data.remove(resource_man.registry.data_ids.item) {
        copied.set(resource_man.registry.data_ids.item, v);
    }

    copied
}
