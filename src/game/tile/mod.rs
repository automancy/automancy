use hexagon_tiles::hex::{hex, Hex};
use hexagon_tiles::traits::HexDirection;
use rune::Module;
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, Mul, Neg, Sub};

pub mod coord;
pub mod entity;

pub type TileHex = Hex<TileUnit>;

pub type TileUnit = i32;
