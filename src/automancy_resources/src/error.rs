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
pub fn error_to_string((id, args): &GameError, resource_man: &ResourceManager) -> String {
    format(
        resource_man.translates.error[id].as_str(),
        args.into_iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
    )
}

/// Gets the unlocalized key of an error's ID.
pub fn error_to_key((id, ..): &GameError, resource_man: &ResourceManager) -> String {
    resource_man.interner.resolve(*id).unwrap().to_string()
}

impl ErrorManager {
    /// Adds a new error to the queue.
    pub fn push(&self, error: GameError, resource_man: &ResourceManager) {
        log::error!("ERR: {}", error_to_key(&error, resource_man));
        self.queue
            .lock()
            .expect("could not write error")
            .push_front(error);
    }

    /// Copies the top error of the queue and returns it, or None if the queue is empty.
    pub fn peek(&self) -> Option<GameError> {
        self.queue
            .lock()
            .expect("could not read error")
            .front()
            .cloned()
    }

    /// Removes the top error off of the stack and returns it or None if the queue is empty.
    pub fn pop(&self) -> Option<GameError> {
        self.queue.lock().expect("could not read error").pop_front()
    }

    /// Returns true if the queue contains errors, otherwise false.
    pub fn has_errors(&self) -> bool {
        !self.queue.lock().unwrap().is_empty()
    }
}
