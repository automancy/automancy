use core::time::Duration;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Instant,
};

use automancy_data::{
    game::{
        coord::{TileCoord, TileCoordBounds},
        generic::DataMap,
    },
    id::{ItemId, TileId},
};
use ractor::{Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use rayon::prelude::*;

/// Game ticks per second
pub const TPS: u64 = 20;
pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
pub const MAX_ALLOWED_TICK_INTERVAL: Duration = TICK_INTERVAL.saturating_mul(5);

pub const TRANSACTION_ANIMATION_SPEED: Duration = Duration::from_nanos(800_000_000);
pub const TRANSACTION_MIN_INTERVAL: Duration = Duration::from_nanos(250_000_000);

const UNDO_CACHE_SIZE: usize = 256;

pub type TickUnit = u16;

use crate::{
    actor,
    actor::{
        FlatTiles, TileEntry, TileMap,
        message::{GameMsg, GameRenderCommands, PlaceTileResponse, TileMsg},
        tile_entity::{TileActor, TileActorError},
        util::multi_call_iter,
    },
    persistent::{map, map::GameMap},
    resources::ResourceManager,
    scripting::render,
};

pub struct GameData {
    /// the number of the ticks that have happened
    tick_count: TickUnit,
    map: GameMap,

    /// what to do to undo the last [`UNDO_CACHE_SIZE`] user events
    undo_steps: arraydeque::ArrayDeque<Vec<GameMsg>, UNDO_CACHE_SIZE, arraydeque::Wrapping>,
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
    cleanup_render_commands: GameRenderCommands,
    render_commands: GameRenderCommands,
    tracked_none_renders: BTreeSet<TileCoord>,
    last_culling_bounds: TileCoordBounds,
}

pub struct GameActor {
    pub resource_man: Arc<ResourceManager>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl GameActor {
    /// Stops a Tile and removes it from the game.
    #[must_use]
    async fn remove_tile(
        &self,
        game_data: &mut GameData,
        coord: TileCoord,
        cleanup_render_commands: &mut GameRenderCommands,
        last_culling_bounds: TileCoordBounds,
    ) -> Option<(TileId, DataMap)> {
        let removed_tile = game_data.map.tiles.remove(&coord);
        let mut removed_data = None;

        if let Some(tile) = &removed_tile {
            // add back build material
            try_category(&self.resource_man, tile.id, |item| {
                let inventory = game_data
                    .map
                    .map_info
                    .data
                    .inventory_mut(self.resource_man.registry.data_ids.player_inventory);

                inventory.add(item, 1);
            });

            // stop rendering if needed
            if last_culling_bounds.contains(coord) {
                cleanup_render_commands
                    .entry(coord)
                    .or_default()
                    .append(&mut tile.handle.call(TileMsg::StopRendering, None).await.unwrap().unwrap());
            }

            // unload tile
            removed_data = Some(tile.handle.call(TileMsg::TakeData, None).await.unwrap().unwrap());
            tile.handle
                .stop_and_wait(Some("removed from game".to_string()), None)
                .await
                .unwrap();
        } else {
            // no tile to unload, remove default none tile if needed
            if last_culling_bounds.contains(coord) {
                cleanup_render_commands
                    .entry(coord)
                    .or_default()
                    .extend_from_slice(&render::util::untrack_none(&self.resource_man));
            }
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

        tiles.insert(
            coord,
            TileEntry {
                id,
                handle: handle.clone(),
            },
        );

        handle
    }

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    async fn place_tile(
        &self,
        myself: ActorRef<GameMsg>,
        game_data: &mut GameData,
        coord: TileCoord,
        render_commands: &mut GameRenderCommands,
        cleanup_render_commands: &mut GameRenderCommands,
        last_culling_bounds: TileCoordBounds,
        (id, data): (TileId, DataMap),
    ) -> Option<(TileId, DataMap)> {
        let mut requirement_unmet = false;

        // take build material if needed
        try_category(&self.resource_man, id, |item| {
            let inventory = game_data
                .map
                .map_info
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

        let removed_tile = self
            .remove_tile(game_data, coord, cleanup_render_commands, last_culling_bounds)
            .await;

        // placing "none" means removing the tile
        if id.is_none() {
            if last_culling_bounds.contains(coord) {
                render_commands
                    .entry(coord)
                    .or_default()
                    .extend_from_slice(&render::util::track_none(&self.resource_man, coord));
            }
            return removed_tile;
        }

        // add new tile
        let handle = self.insert_new_tile(myself.clone(), &mut game_data.map.tiles, coord, id).await;
        handle.cast(TileMsg::SetData(data)).unwrap();

        // track render if in range
        if last_culling_bounds.contains(coord) {
            render_commands
                .entry(coord)
                .or_default()
                .append(&mut handle.call(TileMsg::StartRendering, None).await.unwrap().unwrap());
        }

        removed_tile
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
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
            GameMsg::SaveMap => match &state.game_state {
                GameState::Running(game_data) | GameState::Paused(game_data) => {
                    map::serialize::save_map(&game_data.map, &self.resource_man.interner).await.unwrap();
                },
                _ => {},
            },
            GameMsg::SaveAndUnload(reply) => {
                let mut result = anyhow::Ok(());

                match std::mem::take(&mut state.game_state) {
                    GameState::Running(game_data) | GameState::Paused(game_data) => {
                        result = map::serialize::save_map(&game_data.map, &self.resource_man.interner)
                            .await
                            .map_err(Into::into);
                    },
                    _ => {},
                }

                reply.send(result)?;
            },
            GameMsg::LoadMap(map_id, reply) => {
                match std::mem::take(&mut state.game_state) {
                    GameState::Running(mut game_data) | GameState::Paused(mut game_data) => {
                        for coord in state.last_culling_bounds {
                            if let Some(tile) = game_data.map.tiles.get(&coord) {
                                state
                                    .cleanup_render_commands
                                    .entry(coord)
                                    .or_default()
                                    .append(&mut tile.handle.call(TileMsg::StopRendering, None).await.unwrap().unwrap());
                            } else {
                                state
                                    .cleanup_render_commands
                                    .entry(coord)
                                    .or_default()
                                    .extend(render::util::untrack_none(&self.resource_man));
                                state.tracked_none_renders.remove(&coord);
                            }
                        }

                        for (_coord, tile) in std::mem::take(&mut game_data.map.tiles).into_inner() {
                            tile.handle.stop_and_wait(None, None).await.unwrap();
                        }
                    },
                    _ => {},
                }

                state.last_culling_bounds = TileCoordBounds::Empty;

                let (flat_tiles, map_info) = match map::serialize::load_map(self.resource_man.clone(), map_id.clone()) {
                    Ok(v) => v,
                    Err(_) => {
                        reply.send(false)?;

                        //panic!("TODO: handle map reading error");
                        return Ok(());
                    },
                };

                let mut tiles = TileMap::default();
                for (coord, (id, _)) in &flat_tiles {
                    self.insert_new_tile(myself.clone(), &mut tiles, *coord, *id).await;
                }

                flat_tiles.into_par_iter().for_each(|(coord, (_, data))| {
                    tiles.get(&coord).unwrap().handle.cast(TileMsg::SetData(data)).unwrap();
                });

                log::info!("Successfully loaded map '{map_id}'!");

                state.game_state = GameState::Running(GameData {
                    map: GameMap {
                        id: map_id,
                        tiles,
                        map_info,
                    },
                    tick_count: 0, // TODO should the new GameState inherit the tick count?
                    undo_steps: arraydeque::ArrayDeque::new(),
                });

                reply.send(true)?;
            },
            GameMsg::GetMap(reply) => match &state.game_state {
                GameState::Running(game_data) | GameState::Paused(game_data) => {
                    reply.send(Some((game_data.map.id.clone(), game_data.map.map_info.clone())))?;
                },
                _ => {
                    reply.send(None)?;
                },
            },

            GameMsg::GetMapData(reply) => match &state.game_state {
                GameState::Running(game_data) | GameState::Paused(game_data) => {
                    reply.send(game_data.map.map_info.data.clone())?;
                },
                _ => {
                    reply.send(DataMap::new())?;
                },
            },

            GameMsg::ChangeMapData(changes, reply) => {
                match &mut state.game_state {
                    GameState::Running(game_data) | GameState::Paused(game_data) => {
                        for change in changes {
                            game_data.map.map_info.data.handle_change(change);
                        }
                    },
                    _ => {},
                }

                reply.send(())?;
            },

            message => {
                let in_game = matches!(
                    state.game_state,
                    GameState::Running(GameData {
                        map: GameMap {
                            id: map::GameMapId::SaveFile(..),
                            ..
                        },
                        ..
                    })
                );

                match &mut state.game_state {
                    GameState::Running(game_data) | GameState::Paused(game_data) => match message {
                        GameMsg::SaveMap
                        | GameMsg::SaveAndUnload(..)
                        | GameMsg::LoadMap(..)
                        | GameMsg::GetMap(..)
                        | GameMsg::GetMapData(..)
                        | GameMsg::ChangeMapData(..) => {
                            unreachable!("should be handled earlier")
                        },

                        GameMsg::Tick => {
                            if !in_game {
                                return Ok(());
                            }

                            let start = Instant::now();
                            {
                                game_data.map.tiles.par_iter().for_each(|(_coord, tile)| {
                                    if let Err(e) = tile.handle.send_message(TileMsg::Tick {
                                        tick_count: game_data.tick_count,
                                    }) {
                                        log::error!("{e}");
                                    }
                                });

                                game_data.tick_count = game_data.tick_count.wrapping_add(1);
                            }
                            let finish = Instant::now();

                            {
                                let tick_time = finish - start;

                                if tick_time >= actor::game_entity::MAX_ALLOWED_TICK_INTERVAL {
                                    log::warn!(
                                        "Tick took longer than the allowed maximum! tick_time: {}ms, maximum: {}ms",
                                        tick_time.as_millis(),
                                        actor::game_entity::MAX_ALLOWED_TICK_INTERVAL.as_millis()
                                    );
                                }
                            }
                        },
                        GameMsg::SendTileMsg(coord, msg) => {
                            if !in_game {
                                return Ok(());
                            }

                            if let Some(tile) = game_data.map.tiles.get(&coord) {
                                tile.handle.send_message(msg)?;
                            }
                        },
                        GameMsg::PlaceTile {
                            coord,
                            tile: (id, data),
                            record,
                            reply,
                        } => {
                            if !in_game {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored)?;
                                }
                                return Ok(());
                            }

                            if let Some(old_tile) = game_data.map.tiles.get(&coord)
                                && old_tile.id == id
                            {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored)?;
                                }

                                return Ok(());
                            }

                            if id.is_none() && !game_data.map.tiles.contains_key(&coord) {
                                if let Some(reply) = reply {
                                    reply.send(PlaceTileResponse::Ignored)?;
                                }

                                return Ok(());
                            }

                            let removed_tile = self
                                .place_tile(
                                    myself.clone(),
                                    game_data,
                                    coord,
                                    &mut state.render_commands,
                                    &mut state.cleanup_render_commands,
                                    state.last_culling_bounds,
                                    (id, data),
                                )
                                .await;

                            if let Some(reply) = reply {
                                if let Some((id, _data)) = &removed_tile {
                                    if !id.is_none() {
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
                        },
                        GameMsg::PlaceTiles {
                            tiles,
                            replace,
                            record,
                            reply,
                        } => {
                            let mut removed_tiles = FlatTiles::default();
                            if !in_game {
                                if let Some(reply) = reply {
                                    reply.send(removed_tiles)?;
                                }
                                return Ok(());
                            }

                            for (coord, tile) in tiles.into_iter() {
                                let should_place = replace || game_data.map.tiles.get(&coord).is_none();

                                if should_place
                                    && let Some(tile) = self
                                        .place_tile(
                                            myself.clone(),
                                            game_data,
                                            coord,
                                            &mut state.render_commands,
                                            &mut state.cleanup_render_commands,
                                            state.last_culling_bounds,
                                            tile,
                                        )
                                        .await
                                {
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
                        },
                        GameMsg::MoveTiles(tiles, direction, record) => {
                            if !in_game {
                                return Ok(());
                            }

                            let mut undo = vec![];

                            let mut removed_tiles = Vec::new();

                            for coord in tiles {
                                if let Some(old) = self
                                    .remove_tile(game_data, coord, &mut state.cleanup_render_commands, state.last_culling_bounds)
                                    .await
                                {
                                    removed_tiles.push((coord, old));
                                }
                            }

                            for (coord, tile) in removed_tiles {
                                let new_coord = coord + direction;

                                let _ = self
                                    .place_tile(
                                        myself.clone(),
                                        game_data,
                                        new_coord,
                                        &mut state.render_commands,
                                        &mut state.cleanup_render_commands,
                                        state.last_culling_bounds,
                                        tile,
                                    )
                                    .await;

                                undo.push(new_coord);
                            }

                            if record {
                                game_data.undo_steps.push_back(vec![GameMsg::MoveTiles(undo, -direction, false)]);
                            }
                        },
                        GameMsg::Undo => {
                            if !in_game {
                                return Ok(());
                            }

                            if let Some(step) = game_data.undo_steps.pop_back() {
                                for msg in step {
                                    myself.send_message(msg)?;
                                }
                            }
                        },
                        GameMsg::GetTile(coord, reply) => {
                            reply.send(game_data.map.tiles.get(&coord).cloned())?;
                        },
                        GameMsg::GetTileFlat(coord, reply) => {
                            if let Some(tile) = game_data.map.tiles.get(&coord) {
                                let data = tile.handle.call(TileMsg::GetData, None).await?.unwrap();
                                reply.send(Some((tile.id, data)))?;
                            } else {
                                reply.send(None)?;
                            }
                        },
                        GameMsg::GetTiles(coords, reply) => {
                            let tiles = coords
                                .into_iter()
                                .flat_map(|coord| game_data.map.tiles.get(&coord).map(|tile| (coord, tile.clone())))
                                .collect::<BTreeMap<_, _>>();

                            reply.send(TileMap(tiles))?;
                        },
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
                        },

                        GameMsg::PushRenderCommands {
                            coord,
                            mut commands,
                        } => {
                            state.render_commands.entry(coord).or_default().append(&mut commands);
                        },
                        GameMsg::CollectRenderCommands {
                            culling_bounds,
                            reply,
                        } => {
                            let mut cleanup_render_commands = state.cleanup_render_commands.clone();
                            let mut render_commands = state.render_commands.clone();
                            state.cleanup_render_commands.clear();
                            state.render_commands.clear();

                            let last_culling_bounds = state.last_culling_bounds;
                            state.last_culling_bounds = culling_bounds;

                            if last_culling_bounds != culling_bounds {
                                for coord in last_culling_bounds {
                                    // unload old render
                                    if !culling_bounds.contains(coord) {
                                        if let Some(tile) = game_data.map.tiles.get(&coord) {
                                            state
                                                .cleanup_render_commands
                                                .entry(coord)
                                                .or_default()
                                                .append(&mut tile.handle.call(TileMsg::StopRendering, None).await.unwrap().unwrap());
                                        } else {
                                            cleanup_render_commands
                                                .entry(coord)
                                                .or_default()
                                                .extend(&render::util::untrack_none(&self.resource_man));
                                            state.tracked_none_renders.remove(&coord);
                                        }
                                    }
                                }

                                for coord in culling_bounds {
                                    // load new render
                                    if !last_culling_bounds.contains(coord) {
                                        if let Some(tile) = game_data.map.tiles.get(&coord) {
                                            state
                                                .render_commands
                                                .entry(coord)
                                                .or_default()
                                                .append(&mut tile.handle.call(TileMsg::StartRendering, None).await.unwrap().unwrap());
                                        } else {
                                            render_commands
                                                .entry(coord)
                                                .or_default()
                                                .extend(&render::util::track_none(&self.resource_man, coord));
                                            state.tracked_none_renders.insert(coord);
                                        }
                                    }
                                }
                            }

                            reply.send([cleanup_render_commands, render_commands])?;
                        },
                    },
                    _ => {},
                }
            },
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
                                let _ = self
                                    .remove_tile(game_data, coord, &mut state.cleanup_render_commands, state.last_culling_bounds)
                                    .await;
                            },
                            _ => {},
                        },
                    }
                }
            },
            SupervisionEvent::ActorTerminated(dead_actor, _tile_state, reason) => {
                log::debug!("Tile entity {dead_actor:?} has been removed. Reason: {reason:?}");
            },
            other => {
                log::debug!("Supervision event: {other}")
            },
        }

        Ok(())
    }
}

pub fn try_category(resource_man: &ResourceManager, id: TileId, category_item: impl FnOnce(ItemId)) {
    if id.is_none() {
        return;
    }

    let tile_def = resource_man.registry.tile_defs.get(&id).unwrap();
    if tile_def.data.bool(resource_man.registry.data_ids.default_tile).copied() == Some(true) {
        return;
    }

    let Some(category) = resource_man.registry.category_defs.get(&tile_def.category) else {
        return;
    };

    if !category.item.is_none() {
        category_item(category.item);
    }
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
    if let Some(v) = data.get(resource_man.registry.data_ids.recipe) {
        copied.set(resource_man.registry.data_ids.recipe, v.clone());
    }
    if let Some(v) = data.get(resource_man.registry.data_ids.capacity) {
        copied.set(resource_man.registry.data_ids.capacity, v.clone());
    }
    if let Some(v) = data.get(resource_man.registry.data_ids.item) {
        copied.set(resource_man.registry.data_ids.item, v.clone());
    }

    copied
}
