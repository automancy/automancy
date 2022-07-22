use bytemuck::{Pod, Zeroable};

use super::raw::{CanBeRaw, Raw};

#[derive(Debug, Clone, Copy, Default)]
pub struct Tile {
    pub id_handle: u16,
    pub data_handle: u16,
}

impl CanBeRaw<RawTile> for Tile {}

const SIZE: usize = 3;

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
            bytes: *bytes.split_array_ref::<3>().0,
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
        let first = self.id_handle.to_le_bytes();
        let second = self.data_handle.to_le_bytes();

        let a = first[0];
        let b = second[0];
        let c = (first[1] << 4) | second[1];

        RawTile { bytes: [a, b, c] }
    }
}

impl From<RawTile> for Tile {
    fn from(val: RawTile) -> Self {
        let a = val.bytes[0];
        let b = val.bytes[1];
        let c = val.bytes[2];

        let first = u16::from_be_bytes([(c >> 4), a]);
        let second = u16::from_be_bytes([((c << 4) >> 4), b]);

        Self {
            id_handle: first,
            data_handle: second,
        }
    }
}
