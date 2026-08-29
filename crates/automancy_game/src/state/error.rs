use core::fmt::Debug;
use std::sync::RwLock;

use automancy_data::id::Id;
use interpolator::Formattable;

use crate::{format::FormatContext, resources::ResourceManager};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomancyError {
    pub id: Id,
    pub message: String,
    pub raw_error: String,
}

/// A manager containing a stack of errors to be displayed.
#[derive(Debug)]
pub struct ErrorManager {
    stack: Vec<AutomancyError>,
}

static ERROR_MAN: RwLock<ErrorManager> = RwLock::new(ErrorManager {
    stack: Vec::new(),
});

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl ErrorManager {
    /// Pushes a new error to the stack.
    pub fn push_err<'a, T>(resource_man: &ResourceManager, id: Id, fmt: T)
    where
        T: Debug + Copy + IntoIterator<Item = (&'a str, Formattable<'a>)>,
    {
        let context = FormatContext::from_iter(fmt);
        log::debug!("<Raw> Recording game error: {id}{context}");

        let message = context.format_str(resource_man.translates.error[&id]);
        log::error!("Recording game error: {message}",);

        ERROR_MAN.write().unwrap().stack.push(AutomancyError {
            id,
            message,
            raw_error: context.to_string(),
        });
    }

    /// Removes the top error off of the stack and returns it, or None if the stack is empty.
    pub fn pop_err() -> Option<AutomancyError> {
        ERROR_MAN.write().unwrap().stack.pop()
    }

    /// Clones the top error of the stack and returns it, or None if the stack is empty.
    pub fn peek_err() -> Option<AutomancyError> {
        ERROR_MAN.read().unwrap().stack.last().cloned()
    }
}
