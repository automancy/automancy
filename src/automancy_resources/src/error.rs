use crate::{format, ResourceManager};
use automancy_defs::id::Id;
use automancy_defs::log;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// An ErrorManager contains a queue of errors to be displayed.
#[derive(Default)]
pub struct ErrorManager {
    queue: Arc<Mutex<VecDeque<GameError>>>,
}

pub type GameError = (Id, Vec<String>);

/// Gets the ID of an error along with its arguments and converts it into a human-readable string.
pub fn error_to_string(err: &GameError, resource_man: &ResourceManager) -> String {
    let mut string = resource_man.translates.error[&err.0].to_string();
    string = format(
        &string,
        err.1
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .as_slice(),
    );
    string
}

/// Gets the unlocalized key of an error's ID.
pub fn error_to_key(err: &GameError, resource_man: &ResourceManager) -> String {
    resource_man.interner.resolve(err.0).unwrap().to_string()
}

impl ErrorManager {
    /// Adds a new error to the queue.
    pub fn push(&self, error: GameError, resource_man: &ResourceManager) {
        log::error!("ERR: {}", error_to_key(&error, resource_man));
        self.queue
            .as_ref()
            .lock()
            .expect("could not write error")
            .push_front(error);
    }
    /// Copies the top error off of the queue and returns it, or None if the queue is empty.
    pub fn peek(&self) -> Option<GameError> {
        self.queue
            .as_ref()
            .lock()
            .expect("could not read error")
            .get(0)
            .cloned()
    }
    /// Removes the top error off of the stack and returns it or None if the queue is empty.
    pub fn pop(&self) -> Option<GameError> {
        self.queue
            .as_ref()
            .lock()
            .expect("could not read error")
            .pop_front()
    }
    /// Returns true if the queue contains errors, otherwise false.
    pub fn has_errors(&self) -> bool {
        let lock = self.queue.as_ref().try_lock();
        if let Ok(queue) = lock {
            !queue.is_empty()
        } else {
            false
        }
    }
}
