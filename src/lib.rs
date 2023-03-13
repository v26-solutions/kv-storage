#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use std::{error::Error as StdError, marker::PhantomData};

use serde::{de::DeserializeOwned, Serialize};

pub trait Fallible {
    type Error: StdError;
}

pub trait Serializer: Fallible {
    /// Serialize an item returning the buffer.
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementor.
    fn serialize<T: Serialize>(&mut self, item: &T) -> Result<&[u8], Self::Error>;
}

pub trait Deserializer: Fallible {
    /// Deserialize some bytes.
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementor
    fn deserialize<T: DeserializeOwned>(bytes: Vec<u8>) -> Result<T, Self::Error>;
}

pub trait Write: Fallible {
    /// Write some bytes into storage at the given key.
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementor
    fn write(&mut self, key: &[u8], bytes: &[u8]) -> Result<(), Self::Error>;
}

pub trait Read: Fallible {
    /// Read some bytes from storage at the given key if they exist.
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementor
    fn read(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error>;
}

pub trait HasKey: Fallible {
    /// Check if a key exists in storage.
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementor
    fn has_key(&self, key: &[u8]) -> Result<bool, Self::Error>;
}

pub trait Storage {
    type Serde: Serializer + Deserializer;
    type Repo: Write + Read + HasKey;
    type Error: StdError;

    /// Save an item against the given key.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Serializer encounters an error.
    /// - Write encounters an error.
    fn save<T: Serialize>(&mut self, key: &[u8], item: &T) -> Result<(), Self::Error>;

    /// Load an item for a given key if it exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Read encounters an error.
    /// - Deserializer encounters an error.
    fn may_load<T: DeserializeOwned>(&self, key: &[u8]) -> Result<Option<T>, Self::Error>;

    /// Check if a key exists in storage.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Storage encounters an error.
    fn has_key(&self, key: &[u8]) -> Result<bool, Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error<S, R> {
    #[error(transparent)]
    Serde(S),
    #[error(transparent)]
    Repo(R),
}

#[derive(Default)]
pub struct GenericStorage<Serde, Repo> {
    serde: Serde,
    repo: Repo,
}

impl<Serde, Repo> Storage for GenericStorage<Serde, Repo>
where
    Serde: Serializer + Deserializer,
    Repo: Read + Write + HasKey,
{
    type Serde = Serde;
    type Repo = Repo;
    type Error = Error<Serde::Error, Repo::Error>;

    fn save<T: Serialize>(&mut self, key: &[u8], item: &T) -> Result<(), Self::Error> {
        let buffer = self.serde.serialize(item).map_err(Error::Serde)?;
        self.repo.write(key, buffer).map_err(Error::Repo)?;
        Ok(())
    }

    fn may_load<T: DeserializeOwned>(&self, key: &[u8]) -> Result<Option<T>, Self::Error> {
        let Some(bytes) = self.repo.read(key).map_err(Error::Repo)? else {
            return Ok(None);
        };

        Serde::deserialize(bytes).map(Some).map_err(Error::Serde)
    }

    fn has_key(&self, key: &[u8]) -> Result<bool, Self::Error> {
        self.repo.has_key(key).map_err(Error::Repo)
    }
}

pub struct Item<T> {
    key: &'static [u8],
    _t: PhantomData<T>,
}

impl<T> Item<T> {
    #[must_use]
    pub const fn new(key: &'static [u8]) -> Self {
        Self {
            key,
            _t: PhantomData,
        }
    }

    /// Save the item to storage.
    ///
    /// # Errors
    ///
    /// This function will return an error if the store encounters an error.
    pub fn save<Store: Storage>(&self, store: &mut Store, item: &T) -> Result<(), Store::Error>
    where
        T: Serialize,
    {
        store.save(self.key, item)
    }

    /// Load the item from storage if it exists, otherwise `None`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the store encounters an error.
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
    #[must_use]
    pub const fn new(prefix: &'static [u8]) -> Self {
        Self {
            prefix,
            _k: PhantomData,
            _v: PhantomData,
        }
    }

    /// Save the item for the given key.
    ///
    /// # Errors
    ///
    /// This function will return an error if the store encounters an error.
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

    /// Load the item for the given key if it exists, otherwise `None`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the store encounters an error.
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

    /// Check if a key exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Storage encounters an error.
    pub fn has_key<Store: Storage>(&self, store: &Store, key: &K) -> Result<bool, Store::Error> {
        let composite = [self.prefix, key.as_ref()].concat();
        store.has_key(&composite)
    }
}

#[macro_export]
macro_rules! item {
    ($key:literal) => {
        $crate::Item::new(concat!(module_path!(), "::", $key).as_bytes())
    };
}

#[macro_export]
macro_rules! map {
    ($key:literal) => {
        $crate::Map::new(concat!(module_path!(), "::", $key).as_bytes())
    };
}
