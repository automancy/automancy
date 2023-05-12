use rayon::prelude::*;
use std::time::{Duration, Instant};

use crate::game::tile::entity::TileEntityMsg;
use crate::game::GameState;

pub const TPS: u64 = 30;

pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
pub const MAX_ALLOWED_TICK_INTERVAL: Duration = TICK_INTERVAL.saturating_mul(5);

pub type TickUnit = u16;

fn inner_tick(state: &mut GameState) {
    state.tile_entities.par_iter().for_each(|(_, tile_entity)| {
        if let Err(e) = tile_entity.send_message(TileEntityMsg::Tick {
            resource_man: state.resource_man.clone(),
            tick_count: state.tick_count,
        }) {
            println!("{e}");
        }
    });

    state.tick_count = state.tick_count.overflowing_add(1).0;
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
