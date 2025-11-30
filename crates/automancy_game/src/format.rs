use core::fmt::Display;
use std::{collections::HashMap, ops::Deref};

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

impl<'a> FromIterator<(&'a str, Formattable<'a>)> for FormatContext<'a> {
    fn from_iter<T: IntoIterator<Item = (&'a str, Formattable<'a>)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl FormatContext<'_> {
    pub fn format_str(&self, s: &str) -> String {
        interpolator::format(s, self)
            .unwrap_or_else(|err| panic!("Could not format string! Format string: {s}, error: {err}. Format context: {self}",))
    }
}

impl Display for FormatContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;

        let len = self.0.len();
        for (idx, (name, value)) in self.0.iter().enumerate() {
            f.write_fmt(format_args!(
                "\"{}\": {}",
                name,
                FormatContext::from_iter([("", *value)]).format_str("\"{}\""),
            ))?;
            if idx + 1 != len {
                f.write_str(", ")?;
            }
        }

        f.write_str("}")?;

        Ok(())
    }
}
