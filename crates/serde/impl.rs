use std::{error::Error as StdError, marker::PhantomData};

use serde::{de::DeserializeOwned, Serialize};

pub trait Serializer {
    type Error: StdError;

    fn serialize<'a, T: Serialize>(&'a mut self, item: &T) -> Result<&'a [u8], Self::Error>;
}

pub trait Deserializer {
    type Error: StdError;

    fn deserialize<T: DeserializeOwned>(bytes: Vec<u8>) -> Result<T, Self::Error>;
}

pub trait Storage {
    type Error: StdError;
    type Serde: Serializer + Deserializer;

    fn save<T>(&mut self, key: &[u8], item: &T) -> Result<(), Self::Error>
    where
        T: Serialize;

    fn may_load<T>(&self, key: &[u8]) -> Result<Option<T>, Self::Error>
    where
        T: DeserializeOwned;
}

pub struct Item<T> {
    key: &'static [u8],
    _t: PhantomData<T>,
}

impl<T> Item<T> {
    pub const fn new(key: &'static [u8]) -> Self {
        Self {
            key,
            _t: PhantomData,
        }
    }

    pub fn save<Store: Storage>(&self, store: &mut Store, item: &T) -> Result<(), Store::Error>
    where
        T: Serialize,
    {
        store.save(self.key, item)
    }

    pub fn may_load<Store: Storage>(&self, store: &Store) -> Result<Option<T>, Store::Error>
    where
        T: DeserializeOwned,
    {
        store.may_load::<T>(self.key)
    }
}

pub struct Map<K, V> {
    prefix: &'static [u8],
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

impl<K, V> Map<K, V>
where
    K: AsRef<[u8]>,
{
    pub const fn new(prefix: &'static [u8]) -> Self {
        Self {
            prefix,
            _k: PhantomData,
            _v: PhantomData,
        }
    }

    pub fn save<Store: Storage>(
        &self,
        store: &mut Store,
        key: &K,
        item: &V,
    ) -> Result<(), Store::Error>
    where
        V: Serialize,
    {
        let composite = [self.prefix, key.as_ref()].concat();
        store.save(&composite, item)
    }

    pub fn may_load<Store: Storage>(
        &self,
        store: &Store,
        key: &K,
    ) -> Result<Option<V>, Store::Error>
    where
        V: DeserializeOwned,
    {
        let composite = [self.prefix, key.as_ref()].concat();
        store.may_load::<V>(&composite)
    }
}
