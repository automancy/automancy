use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Default)]
pub struct Pool<K, V>
where
    K: Default + Hash + Eq + Clone,
    V: Default,
{
    vec: Vec<V>,
    map: HashMap<K, usize>,
}

impl<K, V> Serialize for Pool<K, V>
where
    K: Default + Hash + Eq + Clone + Serialize,
    V: Default + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut keys = Vec::with_capacity(self.map.len());

        self.map.iter().for_each(|(k, v)| keys[*v] = k.clone());

        serializer.collect_seq(self.vec.iter().enumerate().map(|(i, v)| (&keys[i], v)))
    }
}

impl<'de, K, V> Deserialize<'de> for Pool<K, V>
where
    K: Default + Hash + Eq + Clone + Deserialize<'de>,
    V: Default + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PoolVisitor<K, V> {
            t: PhantomData<K>,
            e: PhantomData<V>,
        }

        impl<'de, K, V> Visitor<'de> for PoolVisitor<K, V>
        where
            K: Default + Hash + Eq + Clone + Deserialize<'de>,
            V: Default + Deserialize<'de>,
        {
            type Value = Pool<K, V>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a seq containing data in a Pool")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut pool = Pool::with_capacity(seq.size_hint().unwrap_or(16));

                while let Some((index, data)) = seq.next_element::<(K, V)>()? {
                    pool.insert(&index, data);
                }

                Ok(pool)
            }
        }

        deserializer.deserialize_seq(PoolVisitor {
            t: PhantomData,
            e: PhantomData,
        })
    }
}

impl<K, V> Pool<K, V>
where
    K: Default + Hash + Eq + Clone,
    V: Default,
{
    fn insert(&mut self, index: &K, data: V) -> usize {
        self.vec.push(data);

        let len = self.vec.len();
        assert_ne!(len, 0);

        let i = len - 1;

        self.map.insert(index.clone(), i);

        i
    }

    pub fn index_or(&mut self, name: &K, default: V) -> usize {
        if self.map.contains_key(name) {
            self.map[name]
        } else {
            self.insert(name, default)
        }
    }

    pub fn index(&mut self, index: &K) -> usize {
        self.index_or(index, V::default())
    }

    pub fn get(&self, index: &K) -> Option<usize> {
        self.map.get(index).copied()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vec: Vec::with_capacity(capacity),
            map: HashMap::default(),
        }
    }

    pub fn from_iter(source: impl IntoIterator<Item = (K, V)>) -> Self {
        let mut pool = Self::default();

        source.into_iter().for_each(|(name, data)| {
            pool.insert(&name, data);
        });

        pool
    }

    pub fn to_value(&self, i: usize) -> &V {
        &self.vec[i]
    }

    pub fn to_value_mut(&mut self, i: usize) -> &mut V {
        &mut self.vec[i]
    }
}
