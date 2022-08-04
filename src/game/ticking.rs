use std::time::Duration;

pub const TPS: u64 = 80;

pub const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TPS);
pub const MAX_ALLOWED_TICK_INTERVAL: Duration = TICK_INTERVAL.mul_f64(5.0);
