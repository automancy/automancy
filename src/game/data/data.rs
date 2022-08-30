use std::mem::{size_of, size_of_val};

use super::raw::{CanBeRaw, Raw};

pub type SingleData = u32;

#[derive(Debug, Clone, Default)]
pub struct Data(pub Vec<SingleData>);

impl CanBeRaw<RawData> for Data {}

#[derive(Debug, Clone)]
pub struct RawData {
    pub size: u32,
    pub bytes: Vec<u8>,
}

impl Raw for RawData {
    fn to_bytes(self) -> Vec<u8> {
        let mut it = self.size.to_le_bytes().to_vec();
        it.append(&mut self.bytes.clone());

        it
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let size = u32::from_le_bytes(*bytes.split_array_ref::<4>().0);
        Self {
            size,
            bytes: bytes[4..(4 + size as usize)].to_vec(),
        }
    }

    fn convert(bytes: &[u8]) -> Vec<Self>
    where
        Self: Sized,
    {
        let len = bytes.len();

        let mut pos = 0;
        let mut vec = Vec::new();
        loop {
            if pos >= len {
                break;
            }

            let r = Self::from_bytes(&bytes[pos..]);
            pos += size_of_val(&r);

            vec.push(r);
        }

        vec
    }
}

impl Into<RawData> for Data {
    fn into(self) -> RawData {
        let size = (self.0.len() * size_of::<SingleData>()) as u32;

        let bytes = self
            .0
            .into_iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();

        RawData { size, bytes }
    }
}

impl From<RawData> for Data {
    fn from(val: RawData) -> Self {
        let data = val
            .bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
            .collect::<Vec<_>>();

        Self(data)
    }
}
