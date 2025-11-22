use core::time::Duration;
use std::{sync::Arc, time::Instant};

use automancy_data::{
    game::{
        coord::{TileBounds, TileCoord},
        generic::DataMap,
    },
    id::{Id, TileId},
};
use hashbrown::HashMap;
use ractor::{Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use rayon::prelude::*;

/// Game ticks per second
pub const TPS: u64 = 60;
pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
pub const MAX_ALLOWED_TICK_INTERVAL: Duration = TICK_INTERVAL.saturating_mul(5);

pub const TRANSACTION_ANIMATION_SPEED: Duration = Duration::from_nanos(800_000_000);
pub const TRANSACTION_MIN_INTERVAL: Duration = Duration::from_nanos(250_000_000);
pub const TAKE_ITEM_ANIMATION_SPEED: Duration = Duration::from_nanos(300_000_000);

const UNDO_CACHE_SIZE: usize = 256;

pub type TickUnit = u16;

use crate::{
    actor::{
        TileEntry, TileMap,
        message::{GameMsg, PlaceTileResponse, TileMsg},
        tile_entity::{TileActor, TileActorError},
        util::multi_call_iter,
    },
    persistent::{map, map::GameMap},
    resources::ResourceManager,
    scripting::{render, render::RenderCommand},
};

pub struct GameData {
    /// the number of the ticks that have happened
    tick_count: TickUnit,
    map: GameMap,

    /// what to do to undo the last [`UNDO_CACHE_SIZE`] user events
    undo_steps: arraydeque::ArrayDeque<Vec<GameMsg>, UNDO_CACHE_SIZE, arraydeque::Wrapping>,
    cleanup_render_commands: HashMap<TileCoord, Vec<RenderCommand>>,
    last_culling_bounds: TileBounds,
}

#[derive(Default)]
pub enum GameState {
    #[default]
    Unloaded,

    Running(GameData),
    Paused(GameData),
    Stopped,
}

#[derive(Default)]
pub struct GameActorState {
    game_state: GameState,
}

fn fill_rest_of_map_with_none(
    resource_man: &ResourceManager,
    culling_bounds: TileBounds,
    last_culling_bounds: TileBounds,
    commands: &mut HashMap<TileCoord, Vec<RenderCommand>>,
) {
    if culling_bounds == last_culling_bounds {
        return;
    }

    for coord in culling_bounds.into_iter() {
        if !commands.contains_key(&coord) && !last_culling_bounds.contains(coord) {
            commands.insert(coord, render::util::track_none(resource_man, coord).to_vec());
        }
    }

    for coord in last_culling_bounds.into_iter() {
        if !commands.contains_key(&coord) && !culling_bounds.contains(coord) {
            commands.insert(coord, render::util::untrack_none(resource_man).to_vec());
        }
    }
}

pub struct GameActor {
    pub resource_man: Arc<ResourceManager>,
}

impl GameActor {
    /// Stops a Tile and removes it from the game.
    #[must_use]
    async fn remove_tile(&self, game_data: &mut GameData, coord: TileCoord) -> Option<(TileId, DataMap)> {
        let removed_tile = game_data.map.tiles.remove(&coord);
        let mut removed_data = None;

        if let Some(tile) = &removed_tile {
            try_category(&self.resource_man, tile.id, |item| {
                let inventory = game_data
                    .map
                    .map_data
                    .data
                    .inventory_mut(self.resource_man.registry.data_ids.player_inventory);

                inventory.add(item, 1);
            });

            removed_data = Some(tile.handle.call(TileMsg::TakeData, None).await.unwrap().unwrap());

            let mut cleanup = tile
                .handle
                .call(
                    |reply| TileMsg::CollectRenderCommands {
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

            game_data.cleanup_render_commands.entry(coord).or_default().append(&mut cleanup);

            tile.handle.stop(Some("removed from game".to_string()));
        }

        removed_tile.map(|tile| tile.id).zip(removed_data)
    }

    #[inline]
    async fn insert_new_tile(&self, myself: ActorRef<GameMsg>, tiles: &mut TileMap, coord: TileCoord, id: TileId) -> ActorRef<TileMsg> {
        let (handle, _) = Actor::spawn_linked(
            Some(coord.to_minimal_string()),
            TileActor {
                id,
                coord,
                game: myself.clone(),
                resource_man: self.resource_man.clone(),
            },
            (),
            myself.get_cell(),
        )
        .await
        .unwrap();

        tiles.insert(coord, TileEntry { id, handle: handle.clone() });

        handle
    }

    #[must_use]
    async fn place_tile(
        &self,
        myself: ActorRef<GameMsg>,
        game_data: &mut GameData,
        coord: TileCoord,
        (id, data): (TileId, DataMap),
    ) -> Option<(TileId, DataMap)> {
        let mut requirement_unmet = false;

        try_category(&self.resource_man, id, |item| {
            let inventory = game_data
                .map
                .map_data
                .data
                .inventory_mut(self.resource_man.registry.data_ids.player_inventory);

            if inventory.get(item) > 0 {
                inventory.take(item, 1);
            } else {
                requirement_unmet = true;
            }
        });

        if requirement_unmet {
            return None;
        }

        let mut removed_tile = None;
        if let Some((id, data)) = self.remove_tile(game_data, coord).await {
            if id == TileId(self.resource_man.registry.none) {
                return Some((id, data));
            }

            removed_tile = Some((id, data));
        }

        self.insert_new_tile(myself.clone(), &mut game_data.map.tiles, coord, id)
            .await
            .cast(TileMsg::SetData(data))
            .unwrap();

        removed_tile
    }
}

impl Actor for GameActor {
    type Msg = GameMsg;
    type State = GameActorState;
    type Arguments = ();

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, _args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(GameActorState::default())
    }

    async fn post_stop(&self, _myself: ActorRef<Self::Msg>, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        state.game_state = GameState::Stopped;

        Ok(())
    }

    async fn handle(&self, myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match message {
            GameMsg::Tick => {
                if let GameState::Running(game_data) = &mut state.game_state {
                    let start = Instant::now();
                    {
                        for (_coord, tile) in game_data.map.tiles.iter() {
                            if let Err(e) = tile.handle.send_message(TileMsg::Tick {
                                tick_count: game_data.tick_count,
                            }) {
                                log::error!("{e:?}");
                            }
                        }

                        game_data.tick_count = game_data.tick_count.wrapping_add(1);
                    }
                    let finish = Instant::now();

                    {
                        let tick_time = finish - start;

                        if tick_time >= MAX_ALLOWED_TICK_INTERVAL {
                            log::warn!(
                                "Tick took longer than the allowed maximum! tick_time: {:?}, maximum: {:?}",
                                tick_time,
                                MAX_ALLOWED_TICK_INTERVAL
                            );
                        }
                    }
                }
            }

            GameMsg::SendTileMsg(coord, msg) => match &mut state.game_state {
                GameState::Running(game_data) => {
                    if let Some(tile) = game_data.map.tiles.get(&coord) {
                        tile.handle.send_message(msg)?;
                    }
                }
                _ => {
                    log::warn!("Game is not running but is requested to run `SendTileMsg`! This isn't supposed to happen.");
                }
            },

            GameMsg::SaveMap => match &state.game_state {
                GameState::Running(game_data) | GameState::Paused(game_data) => {
                    map::serialize::save_map(&game_data.map, &self.resource_man.interner).await.unwrap();
                }
                _ => {}
            },
            GameMsg::SaveAndUnload(reply) => {
                let mut result = anyhow::Ok(());

                match std::mem::take(&mut state.game_state) {
                    GameState::Running(game_data) | GameState::Paused(game_data) => {
                        result = map::serialize::save_map(&game_data.map, &self.resource_man.interner)
                            .await
                            .map_err(Into::into);
                    }
                    _ => {}
                }

                reply.send(result)?;
            }
            GameMsg::LoadMap(map_id, reply) => {
                let mut cleanup_render_commands = HashMap::new();

                match std::mem::take(&mut state.game_state) {
                    GameState::Running(mut game_data) | GameState::Paused(mut game_data) => {
                        cleanup_render_commands.extend(game_data.cleanup_render_commands.into_iter());

                        for (coord, tile) in std::mem::take(&mut game_data.map.tiles).into_inner() {
                            if let Some(commands) = tile
                                .handle
                                .call(
                                    |reply| TileMsg::CollectRenderCommands {
                                        reply,
                                        loading: false,
                                        unloading: game_data.last_culling_bounds.contains(coord),
                                    },
                                    None,
                                )
                                .await
                                .unwrap()
                                .unwrap()
                            {
                                cleanup_render_commands.entry(coord).or_default().extend(commands);
                            }

                            tile.handle.stop(None);
                        }

                        fill_rest_of_map_with_none(
                            &self.resource_man,
                            TileBounds::Empty,
                            game_data.last_culling_bounds,
                            &mut cleanup_render_commands,
                        );
                    }
                    _ => {}
                }

                let (flat_tiles, map_data) = match map::serialize::load_map(self.resource_man.clone(), &map_id) {
                    Ok(v) => v,
                    Err(_) => {
                        reply.send(false)?;

                        //panic!("TODO: handle map reading error");
                        return Ok(());
                    }
                };

                let mut tiles = TileMap::default();
                for (coord, (id, _)) in &flat_tiles {
                    self.insert_new_tile(myself.clone(), &mut tiles, *coord, *id).await;
                }

                flat_tiles.into_par_iter().for_each(|(coord, (_, data))| {
                    tiles.get(&coord).unwrap().handle.cast(TileMsg::SetData(data)).unwrap();
                });

                log::info!("Successfully loaded map {map_id}!");

                state.game_state = GameState::Running(GameData {
                    map: GameMap { id: map_id, tiles, map_data },
                    cleanup_render_commands,
                    tick_count: 0, // TODO should the new GameState inherit the tick count?
                    undo_steps: arraydeque::ArrayDeque::new(),
                    last_culling_bounds: TileBounds::Empty,
                });

                reply.send(true)?;
            }
            GameMsg::GetMapIdAndData(reply) => match &state.game_state {
                GameState::Running(game_data) | GameState::Paused(game_data) => {
                    reply.send(Some((game_data.map.id.clone(), game_data.map.map_data.clone())))?;
                }
                _ => {
                    reply.send(None)?;
                }
            },

            message => {
                let running = matches!(state.game_state, GameState::Running(..));

                match &mut state.game_state {
                    GameState::Running(game_data) | GameState::Paused(game_data) => match message {
                        GameMsg::PlaceTile {
                            coord,
                            tile: (id, data),
                            record,
                            reply,
                        } if running => {
                            if let Some(old_tile) = game_data.map.tiles.get(&coord)
                                && old_tile.id == id
                            {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored)?;
                                }

                                return Ok(());
                            }

                            if id == TileId(self.resource_man.registry.none) && !game_data.map.tiles.contains_key(&coord) {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored)?;
                                }

                                return Ok(());
                            }

                            let removed_tile = self.place_tile(myself.clone(), game_data, coord, (id, data)).await;

                            if let Some(reply) = reply {
                                if let Some((id, _data)) = &removed_tile {
                                    if *id == TileId(self.resource_man.registry.none) {
                                        reply.send(PlaceTileResponse::Removed)?;
                                    } else {
                                        reply.send(PlaceTileResponse::Placed)?;
                                    }
                                } else {
                                    reply.send(PlaceTileResponse::Placed)?;
                                }
                            }

                            if let Some(tile) = removed_tile
                                && record
                            {
                                game_data.undo_steps.push_back(vec![GameMsg::PlaceTile {
                                    coord,
                                    tile,
                                    record: false,
                                    reply: None,
                                }]);
                            }
                        }
                        GameMsg::PlaceTiles {
                            tiles,
                            replace,
                            record,
                            reply,
                        } if running => {
                            let mut removed_tiles = HashMap::default();

                            for (coord, tile) in tiles.into_iter() {
                                let should_place = replace || game_data.map.tiles.get(&coord).is_none();

                                if should_place && let Some(tile) = self.place_tile(myself.clone(), game_data, coord, tile).await {
                                    removed_tiles.insert(coord, tile);
                                }
                            }

                            if let Some(reply) = reply {
                                reply.send(removed_tiles)?;
                            } else if record {
                                game_data.undo_steps.push_back(vec![GameMsg::PlaceTiles {
                                    tiles: removed_tiles,
                                    replace,
                                    record: false,
                                    reply: None,
                                }]);
                            }
                        }
                        GameMsg::MoveTiles(tiles, direction, record) if running => {
                            let mut undo = vec![];

                            let mut removed_tiles = Vec::new();

                            for coord in tiles {
                                if let Some(old) = self.remove_tile(game_data, coord).await {
                                    removed_tiles.push((coord, old));
                                }
                            }

                            for (coord, tile) in removed_tiles {
                                let new_coord = coord + direction;

                                let _ = self.place_tile(myself.clone(), game_data, new_coord, tile).await;

                                undo.push(new_coord);
                            }

                            if record {
                                game_data.undo_steps.push_back(vec![GameMsg::MoveTiles(undo, -direction, false)]);
                            }
                        }
                        GameMsg::Undo if running => {
                            if let Some(step) = game_data.undo_steps.pop_back() {
                                for msg in step {
                                    myself.send_message(msg)?;
                                }
                            }
                        }

                        GameMsg::GetTile(coord, reply) => {
                            reply.send(game_data.map.tiles.get(&coord).cloned())?;
                        }
                        GameMsg::GetTileFlat(coord, reply) => {
                            if let Some(tile) = game_data.map.tiles.get(&coord) {
                                let data = tile.handle.call(TileMsg::GetData, None).await?.unwrap();
                                reply.send(Some((tile.id, data)))?;
                            } else {
                                reply.send(None)?;
                            }
                        }
                        GameMsg::GetTiles(coords, reply) => {
                            let tiles = coords
                                .into_iter()
                                .flat_map(|coord| game_data.map.tiles.get(&coord).map(|tile| (coord, tile.clone())))
                                .collect::<HashMap<_, _>>();

                            reply.send(TileMap(tiles))?;
                        }
                        GameMsg::GetTilesFlat(coords, reply) => {
                            let tiles = multi_call_iter(
                                coords.len(),
                                coords
                                    .into_iter()
                                    .flat_map(|coord| game_data.map.tiles.get(&coord).map(|tile| ((coord, tile.id), tile.handle.clone()))),
                                |_, reply| TileMsg::GetData(reply),
                                |(coord, id), v| (coord, (id, v)),
                                None,
                            )
                            .await?;

                            reply.send(tiles)?;
                        }
                        GameMsg::GetAllRenderCommands { culling_bounds, reply } => {
                            let last_culling_bounds = game_data.last_culling_bounds;
                            game_data.last_culling_bounds = culling_bounds;

                            let commands = multi_call_iter(
                                game_data.map.tiles.len(),
                                game_data.map.tiles.iter().map(|(coord, tile)| (*coord, tile.handle.clone())),
                                |&coord, reply| {
                                    let loading = culling_bounds.contains(coord) && !last_culling_bounds.contains(coord);
                                    let unloading = last_culling_bounds.contains(coord) && !culling_bounds.contains(coord);

                                    TileMsg::CollectRenderCommands { reply, loading, unloading }
                                },
                                |k, v| (k, v),
                                None,
                            )
                            .await
                            .map(|commands| commands.into_iter().flat_map(|(k, v)| Some(k).zip(v)).collect());

                            match commands {
                                Ok(mut commands) => {
                                    fill_rest_of_map_with_none(&self.resource_man, culling_bounds, last_culling_bounds, &mut commands);

                                    reply.send([std::mem::take(&mut game_data.cleanup_render_commands), commands])?;
                                }
                                Err(err) => {
                                    log::error!("Could not collect render commands! Error: {err:?}");
                                    reply.send([HashMap::default(), HashMap::default()])?;
                                }
                            }
                        }
                        _ => {}
                    },
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
                log::error!("Tile entity {dead_actor:?} panicked, trying to remove. Error: {error}");

                if let Ok(tile_error) = error.downcast::<Box<TileActorError>>() {
                    match **tile_error {
                        TileActorError::NonExistent(coord) => match &mut state.game_state {
                            GameState::Running(game_data) | GameState::Paused(game_data) => {
                                let _ = self.remove_tile(game_data, coord).await;
                            }
                            _ => {}
                        },
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
    if resource_man
        .registry
        .tile_defs
        .get(&id)
        .unwrap()
        .data
        .bool_or_default(resource_man.registry.data_ids.default_tile, false)
    {
        return;
    }

    let Some(category) = resource_man.registry.tile_defs.get(&id).and_then(|tile| tile.category) else {
        return;
    };

    let Some(item) = resource_man.registry.categorie_defs.get(&category).and_then(|category| category.item) else {
        return;
    };

    category_item(item);
}

// TODO replace this with a script
pub fn copy_auxiliary_data(resource_man: &ResourceManager, data: &DataMap) -> DataMap {
    let mut copied = DataMap::default();

    if let Some(v) = data.get(resource_man.registry.data_ids.direction) {
        copied.set(resource_man.registry.data_ids.direction, v.clone());
    }
    if let Some(v) = data.get(resource_man.registry.data_ids.link) {
        copied.set(resource_man.registry.data_ids.link, v.clone());
    }
    if let Some(v) = data.get(resource_man.registry.data_ids.script) {
        copied.set(resource_man.registry.data_ids.script, v.clone());
    }
    if let Some(v) = data.get(resource_man.registry.data_ids.capacity) {
        copied.set(resource_man.registry.data_ids.capacity, v.clone());
    }
    if let Some(v) = data.get(resource_man.registry.data_ids.item) {
        copied.set(resource_man.registry.data_ids.item, v.clone());
    }

    copied
}
