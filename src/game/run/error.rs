use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use rune::Any;

use crate::resource::ResourceManager;
use crate::util::id::{id_static, Id, Interner};

#[derive(Default)]
pub struct ErrorManager {
    queue: Arc<Mutex<VecDeque<GameError>>>,
}

#[derive(Clone, Copy, Any)]
pub struct ErrorIds {
    pub test_error: Id,
    pub invalid_map_data: Id,
}
impl ErrorIds {
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            test_error: id_static("automancy", "test_error").to_id(interner),
            invalid_map_data: id_static("automancy", "invalid_map_data").to_id(interner),
        }
    }
}
pub type GameError = (Id, Vec<String>);
pub fn error_to_string(err: &GameError, resource_man: &ResourceManager) -> String {
    let mut string = resource_man.translates.error[&err.0].to_string();
    for str in err.1.iter() {
        string = string.replacen("{}", str, 1);
    }
    string
}
pub fn error_to_key(err: &GameError, resource_man: &ResourceManager) -> String {
    resource_man.interner.resolve(err.0).unwrap().to_string()
}
impl ErrorManager {
    pub fn push(&self, error: GameError, resource_man: &ResourceManager) {
        log::error!("ERR: {}", error_to_key(&error, resource_man));
        self.queue
            .as_ref()
            .lock()
            .expect("could not write error")
            .push_front(error);
    }
    pub fn peek(&self) -> Option<GameError> {
        self.queue
            .as_ref()
            .lock()
            .expect("could not read error")
            .get(0)
            .cloned()
    }
    pub fn pop(&self) -> Option<GameError> {
        self.queue
            .as_ref()
            .lock()
            .expect("could not read error")
            .pop_front()
    }
    pub fn has_errors(&self) -> bool {
        let lock = self.queue.as_ref().try_lock();
        if let Ok(queue) = lock {
            !queue.is_empty()
        } else {
            false
        }
    }
}
