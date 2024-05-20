use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use arraydeque::{ArrayDeque, Wrapping};
use hashbrown::HashMap;
use ractor::rpc::CallResult;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};
use rayon::prelude::*;
use tokio::sync::Mutex;

use automancy_defs::coord::{TileBounds, TileCoord};
use automancy_defs::id::Id;
use automancy_defs::log;
use automancy_defs::math::{Float, Matrix4, FAR, HEX_GRID_LAYOUT};
use automancy_defs::rendering::InstanceData;
use automancy_resources::data::stack::ItemStack;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::ResourceManager;

use crate::game::GameSystemMessage::*;
use crate::map::{Map, MapInfo, TileEntities};
use crate::tile_entity::{TileEntity, TileEntityMsg};
use crate::util::actor::multi_call_iter;
use crate::{event::EventLoopStorage, tile_entity::TileEntityError};

/// Game ticks per second
pub const TPS: u64 = 30;
pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
pub const MAX_ALLOWED_TICK_INTERVAL: Duration = TICK_INTERVAL.saturating_mul(5);

pub const TRANSACTION_ANIMATION_SPEED: Duration = Duration::from_nanos(800_000_000);
pub const TRANSACTION_MIN_INTERVAL: Duration = Duration::from_nanos(250_000_000);
pub const TAKE_ITEM_ANIMATION_SPEED: Duration = Duration::from_nanos(200_000_000);

pub type TickUnit = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionRecord {
    pub stack: ItemStack,
    pub source_id: Id,
    pub id: Id,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderUnit {
    pub instance: InstanceData,
    pub model_override: Option<Id>,
}

pub type TransactionRecords =
    HashMap<(TileCoord, TileCoord), VecDeque<(Instant, TransactionRecord)>>;

#[derive(Debug)]
pub struct GameSystemState {
    /// a count of all the ticks that have happened
    tick_count: TickUnit,
    /// is the game stopped
    stopped: bool,

    /// the tile entities
    tile_entities: TileEntities,
    /// the map
    map: Map,

    /// what to do to undo the last UNDO_CACHE_SIZE user events
    undo_steps: ArrayDeque<Vec<GameSystemMessage>, 16, Wrapping>,
    /// records transactions to be drawn
    transaction_records: TransactionRecords,
}

pub async fn load_map(
    game: &ActorRef<GameSystemMessage>,
    loop_store: &mut EventLoopStorage,
    map_name: String,
) -> anyhow::Result<()> {
    game.send_message(LoadMap(map_name))?;
    loop_store.map_info = Some(game.call(GetMapInfoAndName, None).await?.unwrap());

    Ok(())
}

/// Represents a message the game receives
#[derive(Debug)]
pub enum GameSystemMessage {
    /// tick the tile once
    Tick,
    StopTicking,
    /// send a message to a tile entity
    ForwardMsgToTile(TileCoord, TileEntityMsg),
    /// place a tile at the given position
    PlaceTile {
        coord: TileCoord,
        id: Id,
        data: Option<DataMap>,
        record: bool,
        reply: Option<RpcReplyPort<PlaceTileResponse>>,
    },
    PlaceTiles {
        tiles: Vec<(TileCoord, Id, Option<DataMap>)>,
        reply: Option<RpcReplyPort<Vec<(TileCoord, Id, Option<DataMap>)>>>,
        place_over: bool,
        record: bool,
    },
    MoveTiles(Vec<TileCoord>, TileCoord, bool),
    Undo,

    /// load a map
    LoadMap(String),
    /// save the map
    SaveMap(RpcReplyPort<()>),

    GetMapInfoAndName(RpcReplyPort<(Arc<Mutex<MapInfo>>, String)>),

    /// get the tile at the given position
    GetTile(TileCoord, RpcReplyPort<Option<Id>>),
    /// get the tile entity at the given position
    GetTileEntity(TileCoord, RpcReplyPort<Option<ActorRef<TileEntityMsg>>>),
    /// get all the data from the tiles
    GetAllData(RpcReplyPort<HashMap<TileCoord, DataMap>>),
    /// get all the tiles needing to be rendered, and their info
    GetAllRenderUnits {
        culling_range: TileBounds,
        reply: RpcReplyPort<HashMap<TileCoord, (Id, RenderUnit)>>,
    },
    GetTiles(
        Vec<TileCoord>,
        RpcReplyPort<Vec<(TileCoord, Id, Option<DataMap>)>>,
    ),

    GetRecordedTransactions(RpcReplyPort<TransactionRecords>),
    RecordTransaction(ItemStack, TileCoord, TileCoord),
}

#[derive(Debug, Copy, Clone)]
pub enum PlaceTileResponse {
    Placed,
    Removed,
    Ignored,
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
            LoadMap(name) => {
                for tile_entity in state.tile_entities.values() {
                    tile_entity
                        .stop_and_wait(Some("Loading new map".to_string()), None)
                        .await
                        .unwrap();
                }

                let (map, tile_entities) =
                    Map::load(myself.clone(), self.resource_man.clone(), &name).await;

                state.map = map;
                state.tile_entities = tile_entities;
                state.transaction_records.clear();
                state.undo_steps.clear();

                log::info!("Successfully loaded map {name}!");
                return Ok(());
            }
            SaveMap(reply) => {
                state
                    .map
                    .save(&self.resource_man.interner, &state.tile_entities)
                    .await;
                log::info!("Saved map {}", state.map.map_name.clone());
                reply.send(())?;
            }
            GetMapInfoAndName(reply) => {
                reply.send((state.map.info.clone(), state.map.map_name.clone()))?;

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
                    GetAllRenderUnits {
                        culling_range,
                        reply,
                    } => {
                        let instances = state
                            .map
                            .tiles
                            .iter()
                            .filter(|(coord, _)| culling_range.is_in_bounds(***coord))
                            .map(|(coord, id)| {
                                let p = HEX_GRID_LAYOUT.hex_to_world_pos((*coord).into());

                                (
                                    *coord,
                                    (
                                        *id,
                                        RenderUnit {
                                            instance: InstanceData::default().with_model_matrix(
                                                Matrix4::from_translation(p.extend(FAR as Float)),
                                            ),
                                            model_override: None,
                                        },
                                    ),
                                )
                            })
                            .collect();

                        reply.send(instances)?;
                    }
                    PlaceTile {
                        coord,
                        id,
                        data,
                        record,
                        reply,
                    } => {
                        if let Some(old_id) = state.map.tiles.get(&coord) {
                            if *old_id == id {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored)?;
                                }

                                return Ok(());
                            }
                        }

                        if id == self.resource_man.registry.none
                            && !state.map.tiles.contains_key(&coord)
                        {
                            if let Some(reply) = reply {
                                reply.send(PlaceTileResponse::Ignored)?;
                            }

                            return Ok(());
                        }

                        let place_result = insert_new_tile(
                            self.resource_man.clone(),
                            myself.clone(),
                            state,
                            coord,
                            id,
                            data,
                        )
                        .await;

                        if place_result.is_none() {
                            if let Some(reply) = reply {
                                reply.send(PlaceTileResponse::Ignored)?;
                            }
                            return Ok(());
                        }

                        if let Some(old_tile) = place_result {
                            if let Some(reply) = reply {
                                if id == self.resource_man.registry.none {
                                    reply.send(PlaceTileResponse::Removed)?;
                                } else {
                                    reply.send(PlaceTileResponse::Placed)?;
                                }
                            }

                            if record {
                                let (id, data) =
                                    old_tile.unwrap_or((self.resource_man.registry.none, None));

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
                        reply.send(state.map.tiles.get(&coord).cloned())?;
                    }
                    GetTileEntity(coord, reply) => {
                        reply.send(state.tile_entities.get(&coord).cloned())?;
                    }
                    ForwardMsgToTile(coord, msg) => {
                        if let Some(tile_entity) = state.tile_entities.get(&coord) {
                            tile_entity.send_message(msg)?;
                        }
                    }
                    StopTicking => {
                        state.stopped = true;
                    }
                    Undo => {
                        if let Some(step) = state.undo_steps.pop_back() {
                            for msg in step {
                                myself.send_message(msg)?;
                            }
                        }
                    }
                    GetRecordedTransactions(reply) => {
                        let mut to_remove = HashMap::new();

                        let now = Instant::now();
                        for (coord, deque) in state.transaction_records.iter() {
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

                        for (coord, v) in to_remove {
                            for _ in 0..v {
                                state
                                    .transaction_records
                                    .get_mut(&coord)
                                    .unwrap()
                                    .pop_front();
                            }
                        }

                        reply.send(state.transaction_records.clone())?;
                    }
                    GetAllData(reply) => {
                        let all_data = multi_call_iter(
                            state.tile_entities.values(),
                            state.tile_entities.len(),
                            TileEntityMsg::GetDataWithCoord,
                            None,
                        )
                        .await?
                        .into_iter()
                        .flat_map(|v| v.success_or(()))
                        .collect();

                        reply.send(all_data)?;
                    }
                    RecordTransaction(stack, source_coord, coord) => {
                        if let Some((source_id, id)) = state
                            .map
                            .tiles
                            .get(&source_coord)
                            .cloned()
                            .zip(state.map.tiles.get(&coord).cloned())
                        {
                            let record = TransactionRecord {
                                stack,
                                source_id,
                                id,
                            };

                            if let Some(transactions) =
                                state.transaction_records.get(&(source_coord, coord))
                            {
                                if transactions.iter().any(|(instant, other)| {
                                    &record == other
                                        || Instant::now().duration_since(*instant)
                                            < TRANSACTION_MIN_INTERVAL
                                }) {
                                    return Ok(());
                                }
                            }

                            state
                                .transaction_records
                                .entry((source_coord, coord))
                                .or_insert_with(Default::default)
                                .push_back((Instant::now(), record));
                        }
                    }
                    GetTiles(coords, reply) => {
                        let mut tiles = vec![];

                        for (id, coord) in coords
                            .into_iter()
                            .flat_map(|coord| state.map.tiles.get(&coord).zip(Some(coord)))
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
                            if place_over || state.map.tiles.get(&coord).is_none() {
                                if let Some(Some((old_id, old_data))) = insert_new_tile(
                                    self.resource_man.clone(),
                                    myself.clone(),
                                    state,
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
                            if let Some(old) = remove_tile(&self.resource_man, state, coord).await {
                                removed.push((coord, old));
                            }
                        }

                        for (coord, (id, data)) in removed {
                            let new_coord = coord + direction;

                            insert_new_tile(
                                self.resource_man.clone(),
                                myself.clone(),
                                state,
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
                            remove_tile(&self.resource_man, state, coord).await;
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

pub fn try_category(resource_man: &ResourceManager, id: Id, category_item: impl FnOnce(Id)) {
    if let Some(Data::Id(category)) = resource_man
        .registry
        .tiles
        .get(&id)
        .and_then(|tile| tile.data.get(&resource_man.registry.data_ids.category))
    {
        if Data::Bool(false)
            == *resource_man.registry.tiles[&id]
                .data
                .get(&resource_man.registry.data_ids.default_tile)
                .unwrap_or(&Data::Bool(false))
        {
            if let Some(item) = resource_man
                .registry
                .categories
                .get(category)
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
    id: Id,
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
    state: &mut GameSystemState,
    coord: TileCoord,
) -> Option<(Id, Option<DataMap>)> {
    if let Some((tile, tile_entity)) = state
        .map
        .tiles
        .remove(&coord)
        .zip(state.tile_entities.remove(&coord))
    {
        {
            let lock = &mut state.map.info.lock().await;

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

        tile_entity
            .stop_and_wait(Some("Removed from game".to_string()), None)
            .await
            .unwrap();

        Some((tile, data))
    } else {
        None
    }
}

/// Makes a new tile and add it into both the map and the game
async fn insert_new_tile(
    resource_man: Arc<ResourceManager>,
    game: ActorRef<GameSystemMessage>,
    state: &mut GameSystemState,
    coord: TileCoord,
    tile_id: Id,
    data: Option<DataMap>,
) -> Option<Option<(Id, Option<DataMap>)>> {
    let mut skip = false;

    {
        let lock = &mut state.map.info.lock().await;

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
        return None;
    }

    let old = remove_tile(&resource_man, state, coord).await;

    if tile_id == resource_man.registry.none {
        return Some(old);
    }

    let tile_entity = new_tile(resource_man.clone(), game, coord, tile_id).await;

    if let Some(data) = data {
        tile_entity
            .send_message(TileEntityMsg::SetData(data))
            .ok()?;
    }

    state.tile_entities.insert(coord, tile_entity);
    state.map.tiles.insert(coord, tile_id);

    Some(old)
}

fn inner_tick(state: &mut GameSystemState) {
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

pub fn copy_auxiliary_data(resource_man: &ResourceManager, data: &mut DataMap) -> DataMap {
    let mut copied = DataMap::default();

    if let Some(v) = data.remove(&resource_man.registry.data_ids.direction) {
        copied.insert(resource_man.registry.data_ids.direction, v);
    }
    if let Some(v) = data.remove(&resource_man.registry.data_ids.link) {
        copied.insert(resource_man.registry.data_ids.link, v);
    }
    if let Some(v) = data.remove(&resource_man.registry.data_ids.script) {
        copied.insert(resource_man.registry.data_ids.script, v);
    }
    if let Some(v) = data.remove(&resource_man.registry.data_ids.amount) {
        copied.insert(resource_man.registry.data_ids.amount, v);
    }
    if let Some(v) = data.remove(&resource_man.registry.data_ids.item) {
        copied.insert(resource_man.registry.data_ids.item, v);
    }

    copied
}

impl Default for GameSystemState {
    fn default() -> Self {
        Self {
            tick_count: 0,
            stopped: false,

            map: Map::new_empty("".to_string()),
            tile_entities: Default::default(),

            undo_steps: Default::default(),
            transaction_records: Default::default(),
        }
    }
}
