use bytemuck::{Pod, Zeroable};

use super::raw::{CanBeRaw, Raw};

#[derive(Debug, Clone, Copy, Default)]
pub struct Tile {
    pub id_handle: u8,
    pub data_handle: u8,
}

impl CanBeRaw<RawTile> for Tile {}

const SIZE: usize = 2;

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct RawTile {
    pub bytes: [u8; SIZE],
}

impl Raw for RawTile {
    fn to_bytes(self) -> Vec<u8> {
        self.bytes.to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            bytes: *bytes.split_array_ref::<SIZE>().0,
        }
    }

    fn convert(bytes: &[u8]) -> Vec<Self>
    where
        Self: Sized,
    {
        bytes
            .array_chunks::<SIZE>()
            .map(|v| Self::from_bytes(v))
            .collect()
    }
}

impl Into<RawTile> for Tile {
    fn into(self) -> RawTile {
        let a = self.id_handle;
        let b = self.data_handle;

        RawTile { bytes: [a, b] }
    }
}

impl From<RawTile> for Tile {
    fn from(val: RawTile) -> Self {
        Self {
            id_handle: val.bytes[0],
            data_handle: val.bytes[1],
        }
    }
}
