use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, Mul, Neg, Sub};

use hexagon_tiles::hex::{hex, Hex};
use hexagon_tiles::traits::HexDirection;
use rune::{Any, Module};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::game::item::ItemAmount;
use crate::util::id::Id;

#[derive(Debug, Clone, Default)]
pub struct Data(pub HashMap<Id, ItemAmount>);

pub type TileHex = Hex<TileUnit>;

pub type TileUnit = i32;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Any)]
pub struct TileCoord(TileHex);

impl From<TileHex> for TileCoord {
    fn from(value: TileHex) -> Self {
        Self(value)
    }
}

impl From<TileCoord> for TileHex {
    fn from(value: TileCoord) -> Self {
        value.0
    }
}

impl TileCoord {
    pub fn install(module: &mut Module) -> Result<(), rune::ContextError> {
        module.ty::<Self>()?;
        module.inst_fn("neg", Self::neg)?;
        module.inst_fn("add", Self::add)?;
        module.inst_fn("sub", Self::sub)?;
        module.inst_fn("mul", Self::mul)?;
        module.inst_fn("div", Self::div)?;
        module.inst_fn("eq", Self::eq)?;
        module.inst_fn("clone", Self::clone)?;
        module.function(["TOP_RIGHT"], || Self::TOP_RIGHT)?;
        module.function(["RIGHT"], || Self::RIGHT)?;
        module.function(["BOTTOM_RIGHT"], || Self::BOTTOM_RIGHT)?;
        module.function(["BOTTOM_LEFT"], || Self::BOTTOM_LEFT)?;
        module.function(["LEFT"], || Self::LEFT)?;
        module.function(["TOP_LEFT"], || Self::TOP_LEFT)?;

        Ok(())
    }

    pub const TOP_RIGHT: Self = Self(TileHex::NEIGHBORS[2]);
    pub const RIGHT: Self = Self(TileHex::NEIGHBORS[3]);
    pub const BOTTOM_RIGHT: Self = Self(TileHex::NEIGHBORS[4]);
    pub const BOTTOM_LEFT: Self = Self(TileHex::NEIGHBORS[5]);
    pub const LEFT: Self = Self(TileHex::NEIGHBORS[0]);
    pub const TOP_LEFT: Self = Self(TileHex::NEIGHBORS[1]);

    pub fn q(self) -> TileUnit {
        self.0.q()
    }

    pub fn r(self) -> TileUnit {
        self.0.r()
    }

    pub fn s(self) -> TileUnit {
        self.0.s()
    }
}

impl TileCoord {
    pub const ZERO: Self = Self(hex(0, 0, 0));

    pub fn new(q: TileUnit, r: TileUnit) -> Self {
        Self(Hex::new(q, r))
    }
}

impl Display for TileCoord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{}, {}]", self.0.q(), self.0.r()))
    }
}

impl Serialize for TileCoord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.0.q())?;
        tuple.serialize_element(&self.0.r())?;
        tuple.end()
    }
}

struct TileCoordVisitor;

impl<'de> Visitor<'de> for TileCoordVisitor {
    type Value = TileCoord;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let q: TileUnit = seq.next_element()?.unwrap();
        let r: TileUnit = seq.next_element()?.unwrap();

        Ok(TileCoord::new(q, r))
    }
}

impl<'de> Deserialize<'de> for TileCoord
where
    Self: Sized,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, TileCoordVisitor)
    }
}

impl Add for TileCoord {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for TileCoord {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<TileUnit> for TileCoord {
    type Output = Self;

    fn mul(self, rhs: TileUnit) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<TileUnit> for TileCoord {
    type Output = Self;

    fn div(self, rhs: TileUnit) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Neg for TileCoord {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
