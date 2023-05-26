use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Default)]
pub struct ErrorManager {
    queue: Arc<Mutex<Vec<&'static str>>>,
}
impl ErrorManager {
    pub fn push(&self, error: &'static str) {
        eprintln!("E: {error}");
        self.queue
            .as_ref()
            .lock()
            .expect("could not write error")
            .push(error);
    }
    pub fn pop(&self) -> Option<&'static str> {
        self.queue
            .as_ref()
            .lock()
            .expect("could not read error")
            .pop()
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
