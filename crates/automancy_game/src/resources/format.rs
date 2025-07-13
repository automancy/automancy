use std::ops::Deref;

use hashbrown::HashMap;
use interpolator::{Context, Formattable};

#[derive(Debug, Clone)]
pub struct FormatContext<'a>(HashMap<&'a str, Formattable<'a>>);

impl<'a> Context for FormatContext<'a> {
    fn get(&self, key: &str) -> Option<Formattable<'a>> {
        self.0.get(key).cloned()
    }
}

impl<'a> Deref for FormatContext<'a> {
    type Target = HashMap<&'a str, Formattable<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: Iterator<Item = (&'a str, Formattable<'a>)>> From<T> for FormatContext<'a> {
    fn from(value: T) -> Self {
        Self(value.collect())
    }
}

pub fn format_str(s: &str, fmt: &FormatContext) -> String {
    interpolator::format(s, fmt).unwrap_or_else(|err| {
        panic!(
            "Could not format string! Format string: {s}, error: {err:?}. Available variables: {fmt:?}",
        )
    })
}
