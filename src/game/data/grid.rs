use std::{any::type_name, marker::PhantomData};

use super::raw::{CanBeRaw, Raw};

pub const GRID_SIZE: usize = 16 * 16 * 16;

#[derive(Debug)]
pub struct Grid<T, R>
where
    T: CanBeRaw<R>,
    R: Raw,
{
    __phantom: PhantomData<R>,

    pub data: Vec<T>,
}

impl<T, R> Grid<T, R>
where
    T: CanBeRaw<R>,
    R: Raw,
{
    pub fn new(data: Vec<T>) -> Self {
        Self {
            data,
            __phantom: PhantomData,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::with_capacity(GRID_SIZE))
    }
}

impl<T: Default + Clone, R> Default for Grid<T, R>
where
    T: CanBeRaw<R>,
    R: Raw,
{
    fn default() -> Self {
        let mut data = Vec::with_capacity(GRID_SIZE);

        data.resize_with(GRID_SIZE, T::default);

        Self::new(data)
    }
}

impl<T, R> Into<Vec<u8>> for Grid<T, R>
where
    T: CanBeRaw<R> + Default + Clone,
    R: Raw,
{
    fn into(self) -> Vec<u8> {
        let mut data = self.data;

        if !(data.len() == GRID_SIZE) {
            // todo try from
            log::error!(
                "[{}] array vec is not full before converting to bytes",
                type_name::<Self>()
            );
            let mut new = Grid::default().data;
            new.clone_from(&data);
            data = new;
        }

        data.into_iter()
            .flat_map(|v| {
                let raw: R = v.into();

                raw.to_bytes().to_vec()
            })
            .collect()
    }
}
