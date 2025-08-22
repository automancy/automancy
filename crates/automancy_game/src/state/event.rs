use core::{sync::atomic::AtomicBool, time::Duration};
use std::{
    sync::{Arc, Mutex},
    time::{Instant, SystemTime},
};

use automancy_data::id::Id;
use hashbrown::HashMap;
use ractor::ActorRef;

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
    pub config_open_updating: Arc<AtomicBool>,
    pub pointing_cache: Arc<Mutex<Option<TileEntry>>>,
    pub pointing_updating: Arc<AtomicBool>,
}
