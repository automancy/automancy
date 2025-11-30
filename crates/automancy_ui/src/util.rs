use crate::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StaticList<T: 'static> {
    StaticSlice(&'static [T]),
    Rc(Rc<[T]>),
    Arc(Arc<[T]>),
}

impl<T> Default for StaticList<T> {
    fn default() -> Self {
        StaticList::StaticSlice(&[])
    }
}

impl<T> Deref for StaticList<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            StaticList::StaticSlice(v) => v,
            StaticList::Rc(v) => v.as_ref(),
            StaticList::Arc(v) => v.as_ref(),
        }
    }
}

impl<T> StaticList<T> {
    pub fn slice(v: &'static [T]) -> StaticList<T> {
        StaticList::StaticSlice(v)
    }

    pub fn rc(v: impl Into<Rc<[T]>>) -> StaticList<T> {
        StaticList::Rc(v.into())
    }

    pub fn arc(v: impl Into<Arc<[T]>>) -> StaticList<T> {
        StaticList::Arc(v.into())
    }
}

impl<T> From<&'static [T]> for StaticList<T> {
    fn from(value: &'static [T]) -> Self {
        StaticList::StaticSlice(value)
    }
}

impl<T> From<Rc<[T]>> for StaticList<T> {
    fn from(value: Rc<[T]>) -> Self {
        StaticList::Rc(value)
    }
}

impl<T> From<Arc<[T]>> for StaticList<T> {
    fn from(value: Arc<[T]>) -> Self {
        StaticList::Arc(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusState {
    #[default]
    None,
    Clear,
    Set(WidgetId),
}
