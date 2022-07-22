use std::fmt::{Display, Formatter};

pub type Real = i32;

#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Pos(pub Real, pub Real);

impl Display for Pos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{},{}",
            self.0.to_string(),
            self.1.to_string()
        ))
    }
}
