use std::{
    fmt::{Display, Formatter},
    ops::{Add, Sub},
};

pub type Real = i32;

#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub struct Pos(pub Real, pub Real);

impl Sub<Pos> for Pos {
    type Output = Self;

    fn sub(self, rhs: Pos) -> Self::Output {
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl Add<Pos> for Pos {
    type Output = Self;

    fn add(self, rhs: Pos) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Display for Pos {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{},{}",
            self.0.to_string(),
            self.1.to_string()
        ))
    }
}
