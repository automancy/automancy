mod immutable_map {
    use core::{fmt::Debug, marker::PhantomData, ops::Index};
    use std::sync::Arc;

    use crate::{
        id::{Id, IdLike},
        id_map::IdMap,
    };

    type InnerImmutableMap<K, V> = boomphf::hashmap::BoomHashMap<K, V>;

    #[derive(Debug, Clone)]
    pub struct ImmutableIdMap<K, V>
    where
        K: IdLike,
    {
        __: PhantomData<K>,
        inner: Arc<InnerImmutableMap<Id, V>>,
    }

    impl<K, V> Default for ImmutableIdMap<K, V>
    where
        K: IdLike,
    {
        fn default() -> Self {
            Self {
                __: PhantomData,
                inner: Arc::new(InnerImmutableMap::from_iter([])),
            }
        }
    }

    impl<K, V> ImmutableIdMap<K, V>
    where
        K: IdLike,
    {
        pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
            self.inner.iter().map(|(key, value)| (K::from(*key), value))
        }

        pub fn is_empty(&self) -> bool {
            self.inner.is_empty()
        }

        pub fn len(&self) -> usize {
            self.inner.len()
        }

        pub fn get(&self, id: &K) -> Option<&V> {
            self.inner.get(&(*id).into())
        }

        pub fn contains_key(&self, id: &K) -> bool {
            self.get(id).is_some()
        }
    }

    impl<K, V> Index<&K> for ImmutableIdMap<K, V>
    where
        K: IdLike,
    {
        type Output = V;

        fn index(&self, index: &K) -> &Self::Output {
            self.get(index).unwrap()
        }
    }

    impl<K, V> From<IdMap<K, V>> for ImmutableIdMap<K, V>
    where
        K: IdLike,
    {
        fn from(value: IdMap<K, V>) -> Self {
            Self::from_iter(value)
        }
    }

    impl<K, V> FromIterator<(K, V)> for ImmutableIdMap<K, V>
    where
        K: IdLike,
    {
        fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
            Self {
                __: PhantomData,
                inner: Arc::new(InnerImmutableMap::from_iter(
                    iter.into_iter().map(|(key, value)| (key.into(), value)),
                )),
            }
        }
    }
}

mod immutable_set {
    use crate::{
        id::IdLike,
        id_map::{IdSet, ImmutableIdMap},
    };

    #[derive(Debug, Clone)]
    pub struct ImmutableIdSet<K>(ImmutableIdMap<K, ()>)
    where
        K: IdLike;

    impl<K> Default for ImmutableIdSet<K>
    where
        K: IdLike,
    {
        fn default() -> Self {
            Self(ImmutableIdMap::default())
        }
    }

    impl<K> ImmutableIdSet<K>
    where
        K: IdLike,
    {
        pub fn iter(&self) -> impl Iterator<Item = K> {
            self.0.iter().map(|(key, _)| key)
        }

        pub fn is_empty(&self) -> bool {
            self.0.is_empty()
        }

        pub fn len(&self) -> usize {
            self.0.len()
        }

        pub fn contains(&self, id: &K) -> bool {
            self.0.get(id).is_some()
        }
    }

    impl<K> From<IdSet<K>> for ImmutableIdSet<K>
    where
        K: IdLike,
    {
        fn from(value: IdSet<K>) -> Self {
            Self::from_iter(value.inner)
        }
    }

    impl<K> FromIterator<K> for ImmutableIdSet<K>
    where
        K: IdLike,
    {
        fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
            Self(ImmutableIdMap::from_iter(iter.into_iter().map(|key| (key, ()))))
        }
    }
}

mod map {
    use core::ops::{Deref, DerefMut};

    use hashbrown::{HashMap, hash_map};

    use crate::id::{Id, IdLike};

    type InnerMutableMap<K, V> = HashMap<K, V, nohash_hasher::BuildNoHashHasher<Id>>;

    #[derive(Debug, Clone)]
    pub struct IdMap<K, V>
    where
        K: IdLike,
    {
        pub(crate) inner: InnerMutableMap<K, V>,
    }

    impl<K, V> Default for IdMap<K, V>
    where
        K: IdLike,
    {
        fn default() -> Self {
            Self {
                inner: Default::default(),
            }
        }
    }

    impl<K, V> IdMap<K, V>
    where
        K: IdLike,
    {
        pub fn into_keys(self) -> hash_map::IntoKeys<K, V> {
            self.inner.into_keys()
        }

        pub fn into_values(self) -> hash_map::IntoValues<K, V> {
            self.inner.into_values()
        }
    }

    impl<K, V> IdMap<K, V>
    where
        K: IdLike,
    {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl<K, V> Deref for IdMap<K, V>
    where
        K: IdLike,
    {
        type Target = InnerMutableMap<K, V>;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl<K, V> DerefMut for IdMap<K, V>
    where
        K: IdLike,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.inner
        }
    }

    impl<K, V> IntoIterator for IdMap<K, V>
    where
        K: IdLike,
    {
        type Item = (K, V);
        type IntoIter = <InnerMutableMap<K, V> as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            self.inner.into_iter()
        }
    }

    impl<'a, K, V> IntoIterator for &'a IdMap<K, V>
    where
        K: IdLike,
    {
        type Item = (&'a K, &'a V);
        type IntoIter = <&'a InnerMutableMap<K, V> as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
    }

    impl<K, V> FromIterator<(K, V)> for IdMap<K, V>
    where
        K: IdLike,
    {
        fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
            Self {
                inner: InnerMutableMap::from_iter(iter),
            }
        }
    }
}

mod set {
    use core::ops::{Deref, DerefMut};

    use hashbrown::HashSet;

    use crate::id::{Id, IdLike};

    type InnerMutableSet<K> = HashSet<K, nohash_hasher::BuildNoHashHasher<Id>>;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct IdSet<K>
    where
        K: IdLike,
    {
        pub(crate) inner: InnerMutableSet<K>,
    }

    impl<K> Default for IdSet<K>
    where
        K: IdLike,
    {
        fn default() -> Self {
            Self {
                inner: Default::default(),
            }
        }
    }

    impl<K> IdSet<K>
    where
        K: IdLike,
    {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl<K> Deref for IdSet<K>
    where
        K: IdLike,
    {
        type Target = InnerMutableSet<K>;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl<K> DerefMut for IdSet<K>
    where
        K: IdLike,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.inner
        }
    }

    impl<K> IntoIterator for IdSet<K>
    where
        K: IdLike,
    {
        type Item = K;
        type IntoIter = <InnerMutableSet<K> as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            self.inner.into_iter()
        }
    }

    impl<'a, K> IntoIterator for &'a IdSet<K>
    where
        K: IdLike,
    {
        type Item = &'a K;
        type IntoIter = <&'a InnerMutableSet<K> as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
    }

    impl<K> FromIterator<K> for IdSet<K>
    where
        K: IdLike,
    {
        fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
            Self {
                inner: InnerMutableSet::from_iter(iter),
            }
        }
    }
}

pub use immutable_map::*;
pub use immutable_set::*;
pub use map::*;
pub use set::*;
