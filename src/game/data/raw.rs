pub trait Raw {
    fn from_bytes(bytes: &[u8]) -> Self;
    fn to_bytes(self) -> Vec<u8>;

    fn convert(bytes: &[u8]) -> Vec<Self>
    where
        Self: Sized;
}

pub trait CanBeRaw<R: Raw>
where
    Self: From<R>,
    Self: Into<R>,
{
    fn map_to_reals(val: &[u8]) -> Vec<Self> {
        R::convert(val).into_iter().map(Self::from).collect()
    }
}
