use std::time::Duration;

pub const TPS: u64 = 80;

pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
