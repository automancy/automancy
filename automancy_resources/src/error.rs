use std::cell::RefCell;

use automancy_defs::id::Id;

use crate::{format::FormatContext, ResourceManager};

/// An ErrorManager contains a queue of errors to be displayed.
#[derive(Default)]
struct ErrorManager {
    queue: Vec<(Id, String)>,
}

thread_local! {
    static ERROR_MAN: RefCell<ErrorManager> = RefCell::new(ErrorManager::default());
}

/// Gets the unlocalized key of an error's ID.
pub fn error_to_key(id: Id, resource_man: &ResourceManager) -> &str {
    resource_man.interner.resolve(id).unwrap_or("")
}

/// Adds a new error to the queue.
pub fn push_err(id: Id, fmt: &FormatContext, resource_man: &ResourceManager) {
    log::error!("Recording game error: {}", error_to_key(id, resource_man));

    let string = interpolator::format(&resource_man.translates.error[&id], fmt)
        .expect("could not format error!");

    ERROR_MAN.with_borrow_mut(|error_man| error_man.queue.push((id, string)))
}

/// Removes the top error off of the stack and returns it or None if the queue is empty.
pub fn pop_err() -> Option<(Id, String)> {
    ERROR_MAN.with_borrow_mut(|error_man| error_man.queue.pop())
}

/// Copies the top error of the queue and returns it, or None if the queue is empty.
pub fn peek_err() -> Option<(Id, String)> {
    ERROR_MAN.with_borrow(|error_man| error_man.queue.last().cloned())
}

/// Returns true if the queue contains errors, otherwise false.
pub fn has_err() -> bool {
    ERROR_MAN.with_borrow(|error_man| error_man.queue.is_empty())
}
