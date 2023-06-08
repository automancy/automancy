use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, Local, Utc};

pub mod actor;
pub mod cg;
pub mod colors;
pub mod discord;
pub mod id;

pub fn format(format: &str, args: &[&str]) -> String {
    let mut string = format.to_string();
    for arg in args {
        string = string.replacen("{}", arg, 1);
    }
    string
}

pub fn unix_to_formatted_time(utc: i64, fmt: String) -> String {
    let from_epoch = UNIX_EPOCH + Duration::from_secs(utc as u64);
    let past = DateTime::<Utc>::from(from_epoch);
    let time = DateTime::<Local>::from(past);
    time.format(fmt.as_str()).to_string()
}
