use std::sync::{Arc, Mutex, MutexGuard, Weak};

use tokio::sync::broadcast::{channel, Receiver, Sender};

use super::data::map::Map;

#[derive(Debug, Clone, Copy)]
pub struct Ticked;

#[derive(Debug, Clone, Copy)]
pub struct GameState {
    pub tick_count: usize,
}

pub struct Game {
    map: Arc<Mutex<Map>>,

    tick_count: usize,

    send_game_state: Sender<GameState>,
}

impl Game {
    fn map(&mut self) -> MutexGuard<Map> {
        self.map.lock().unwrap()
    }

    pub fn map_ref(&self) -> Arc<Mutex<Map>> {
        self.map.clone()
    }

    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.overflowing_add(1).0;

        let state = GameState {
            tick_count: self.tick_count,
        };

        self.send_game_state.send(state).unwrap();
    }

    pub fn new(map: Map) -> (Self, Receiver<GameState>) {
        let (send_game_state, recv_game_state) = channel(2);

        let it = Self {
            map: Arc::new(Mutex::new(map)),

            tick_count: 0,

            send_game_state,
        };

        (it, recv_game_state)
    }
}
