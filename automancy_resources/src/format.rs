use std::ops::Deref;

use hashbrown::HashMap;
use interpolator::Context;

pub use interpolator::Formattable;

#[derive(Debug, Clone)]
pub struct FormatContext<'a>(HashMap<&'a str, Formattable<'a>>);

impl<'a> Context for FormatContext<'a> {
    fn get(&self, key: &str) -> Option<Formattable> {
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

pub fn format_str<const LEN: usize>(s: &str, fmt: [(&str, Formattable); LEN]) -> String {
    interpolator::format(s, &FormatContext::from(fmt.into_iter())).unwrap_or_else(|err| {
        panic!(
            "Could not format string! Format string: {s}, error: {err:?}. Available variables: {:?}",
            fmt,
        )
    })
}
