use crate::game::tile::entity::TileEntityMsg;
use crate::game::Game;
use std::time::{Duration, Instant};

pub const TPS: u64 = 30;

pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
pub const MAX_ALLOWED_TICK_INTERVAL: Duration = TICK_INTERVAL.mul_f64(5.0);

pub type TickUnit = u16;

#[derive(Debug, Clone)]
pub struct Ticked;

impl Game {
    fn inner_tick(&mut self) {
        for (tile, _, _) in self.map.lock().unwrap().tiles.values() {
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

    /// Runs the game for one tick, logging if the tick is too long.
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
