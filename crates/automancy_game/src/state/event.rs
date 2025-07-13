use core::time::Duration;
use std::{
    sync::Arc,
    time::{Instant, SystemTime},
};

use automancy_data::id::Id;
use hashbrown::HashMap;
use ractor::ActorRef;
use tokio::sync::Mutex;

use crate::{
    actor::{
        TileEntry,
        map::{GameMapData, GameMapId, serialize::GameMapDataRaw},
        message::TileMsg,
    },
    resources::types::item::ItemDef,
};

/// Stores information that lives for the entire lifetime of the session, and is not dropped at the end of one event cycle or handled elsewhere.
#[derive(Debug, Default)]
pub struct EventLoopStorage {
    /// tag searching cache
    pub tag_cache: HashMap<Id, Arc<Vec<ItemDef>>>,
    /// the last frame's starting time
    pub frame_start: Option<Instant>,
    /// the elapsed time between each frame
    pub elapsed: Duration,

    pub map_infos_cache: Vec<((GameMapDataRaw, Option<SystemTime>), String)>,
    pub map_info: Option<(GameMapId, GameMapData)>,

    pub config_open_cache: Arc<Mutex<Option<ActorRef<TileMsg>>>>,
    pub pointing_cache: Arc<Mutex<Option<TileEntry>>>,
}
