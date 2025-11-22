use core::fmt::Debug;
use std::cell::RefCell;

use automancy_data::id::Id;

use crate::{format::FormatContext, resources::ResourceManager};

/// A manager containing a stack of errors to be displayed.
#[derive(Default)]
pub struct ErrorManager {
    stack: Vec<(Id, String)>,
}

thread_local! {
    static ERROR_MAN: RefCell<ErrorManager> = RefCell::new(ErrorManager::default());
}

impl ErrorManager {
    /// Gets the unlocalized key of an error's ID.
    pub fn error_to_key(id: Id, resource_man: &ResourceManager) -> &str {
        resource_man.interner.resolve(id).unwrap_or("")
    }

    /// Pushes a new error to the stack.
    pub fn push_err<'a, T>(resource_man: &ResourceManager, id: Id, fmt: T)
    where
        FormatContext<'a>: From<T>,
        T: Debug,
    {
        let key = Self::error_to_key(id, resource_man);
        log::debug!("<Raw> Recording game error: {key} - {fmt:?}");

        let error = FormatContext::from(fmt).format_str(&resource_man.translates.error[&id]);
        log::error!("Recording game error: {error}",);

        ERROR_MAN.with_borrow_mut(|error_man| error_man.stack.push((id, error)))
    }

    /// Removes the top error off of the stack and returns it, or None if the stack is empty.
    pub fn pop_err() -> Option<(Id, String)> {
        ERROR_MAN.with_borrow_mut(|error_man| error_man.stack.pop())
    }

    /// Clones the top error of the stack and returns it, or None if the stack is empty.
    pub fn peek_err() -> Option<(Id, String)> {
        ERROR_MAN.with_borrow(|error_man| error_man.stack.last().cloned())
    }

    /// Returns true if the stack contains errors.
    pub fn has_err() -> bool {
        ERROR_MAN.with_borrow(|error_man| !error_man.stack.is_empty())
    }
}
