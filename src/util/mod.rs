use chrono::{DateTime, FixedOffset, Local, Utc};
use std::time::{Duration, UNIX_EPOCH};

pub mod actor;
pub mod cg;
pub mod colors;
pub mod discord;
pub mod id;

/// like format!, but does not require the format string to be static.
pub fn format(format: &str, args: &[&str]) -> String {
    let mut string = format.to_string();
    for arg in args {
        string = string.replacen("{}", arg, 1);
    }
    string
}
/// Converts a UTC Unix timestamp into a formatted time string, using the given strftime format string.
pub fn unix_to_formatted_time(utc: i64, fmt: String) -> String {
    let from_epoch = UNIX_EPOCH + Duration::from_secs(utc as u64);
    let past = DateTime::<Utc>::from(from_epoch);
    let time = DateTime::<Local>::from(past);
    time.format(fmt.as_str()).to_string()
}
