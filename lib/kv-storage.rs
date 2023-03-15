#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use std::{error::Error as StdError, marker::PhantomData};

use serde::{de::DeserializeOwned, Serialize};

pub trait Fallible {
    type Error: StdError;
}

pub trait Serializer: Fallible {
    /// Serialize an item, returning the buffer.
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

pub trait Remove: Fallible {
    /// Remove a key and any associated data from storage.
    ///
    /// # Errors
    ///
    /// This function will return an error depending on the implementor
    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Error>;
}

pub trait Storage: Fallible {
    type Serde: Deserializer;
    type Repo: Read + HasKey;

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

pub trait MutStorage: Storage {
    /// Save an item against the given key.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Serializer encounters an error.
    /// - Write encounters an error.
    fn save<T: Serialize>(&mut self, key: &[u8], item: &T) -> Result<(), Self::Error>;

    /// Remove a key and any associated data from storage.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Storage encounters an error.
    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error<S, R> {
    #[error(transparent)]
    Serde(S),
    #[error(transparent)]
    Repo(R),
}

#[derive(Default)]
pub struct KvStore<Serde, Repo> {
    serde: Serde,
    repo: Repo,
}

impl<Serde, Repo> Fallible for KvStore<Serde, Repo>
where
    Serde: Fallible,
    Repo: Fallible,
{
    type Error = Error<Serde::Error, Repo::Error>;
}

impl<Serde, Repo> Storage for KvStore<Serde, Repo>
where
    Serde: Deserializer,
    Repo: Read + HasKey,
{
    type Serde = Serde;
    type Repo = Repo;

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

impl<Serde, Repo> MutStorage for KvStore<Serde, Repo>
where
    Serde: Serializer + Deserializer,
    Repo: Write + Remove + Read + HasKey,
{
    fn save<T: Serialize>(&mut self, key: &[u8], item: &T) -> Result<(), Self::Error> {
        let buffer = self.serde.serialize(item).map_err(Error::Serde)?;
        self.repo.write(key, buffer).map_err(Error::Repo)?;
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Error> {
        self.repo.remove(key).map_err(Error::Repo)
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
    pub fn save<Store: MutStorage>(&self, store: &mut Store, item: &T) -> Result<(), Store::Error>
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

    /// Check if the item is empty
    ///
    /// # Errors
    ///
    /// This function will return an error if the store encounters an error.
    pub fn is_empty<Store: Storage>(&self, store: &Store) -> Result<bool, Store::Error>
    where
        T: DeserializeOwned,
    {
        store.has_key(self.key).map(|has_key| !has_key)
    }

    /// Clear the item from storage.
    ///
    /// # Errors
    ///
    /// This function will return an error if the store encounters an error.
    pub fn clear<Store: MutStorage>(&self, store: &mut Store) -> Result<(), Store::Error> {
        store.remove(self.key)
    }
}

pub trait WriteKeyPart {
    fn write_key_part(&mut self, part: &[u8]);
}

pub trait WriteCompositeKey {
    fn total_len(&self) -> usize;

    fn write_into<W: WriteKeyPart>(&self, writer: &mut W);
}

pub struct Map<const N: usize, K, V> {
    prefix: &'static [u8],
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

impl<const N: usize, K, V> Map<N, K, V>
where
    K: WriteCompositeKey,
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
    pub fn save<Store: MutStorage>(
        &self,
        store: &mut Store,
        key: &K,
        item: &V,
    ) -> Result<(), Store::Error>
    where
        V: Serialize,
    {
        let composite = compose_key::<N>(self.prefix, key);
        store.save(composite.as_ref(), item)
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
        let composite = compose_key::<N>(self.prefix, key);
        store.may_load::<V>(composite.as_ref())
    }

    /// Check if a key exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Storage encounters an error.
    pub fn has_key<Store: Storage>(&self, store: &Store, key: &K) -> Result<bool, Store::Error> {
        let composite = compose_key::<N>(self.prefix, key);
        store.has_key(composite.as_ref())
    }

    /// Remove any item stored at the given key.
    ///
    /// # Errors
    ///
    /// This function will return an error if the store encounters an error.
    pub fn remove<Store: MutStorage>(
        &self,
        store: &mut Store,
        key: &K,
    ) -> Result<(), Store::Error> {
        let composite = compose_key::<N>(self.prefix, key);
        store.remove(composite.as_ref())
    }
}

enum CompositeKeyBuffer<const N: usize> {
    Stack { buffer: [u8; N], len: usize },
    Heap(Box<[u8]>),
}

impl<const N: usize> AsRef<[u8]> for CompositeKeyBuffer<N> {
    fn as_ref(&self) -> &[u8] {
        match self {
            CompositeKeyBuffer::Stack { buffer, len } => &buffer[..*len],
            CompositeKeyBuffer::Heap(v) => v.as_ref(),
        }
    }
}

impl<const N: usize> AsMut<[u8]> for CompositeKeyBuffer<N> {
    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            CompositeKeyBuffer::Stack { buffer, len } => &mut buffer[..*len],
            CompositeKeyBuffer::Heap(v) => v.as_mut(),
        }
    }
}

impl<const N: usize> AsRef<[u8]> for CompositeKey<N> {
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_ref()
    }
}

struct CompositeKey<const N: usize> {
    buffer: CompositeKeyBuffer<N>,
    written: usize,
}

impl<const N: usize> CompositeKey<N> {
    fn new(len: usize) -> Self {
        let buffer = if len > N {
            CompositeKeyBuffer::Heap(vec![0; len].into_boxed_slice())
        } else {
            CompositeKeyBuffer::Stack {
                buffer: [0; N],
                len,
            }
        };

        Self { buffer, written: 0 }
    }
}

impl<const N: usize> WriteKeyPart for CompositeKey<N> {
    fn write_key_part(&mut self, part: &[u8]) {
        self.buffer.as_mut()[self.written..self.written + part.len()].copy_from_slice(part);
        self.written += part.len();
    }
}

fn compose_key<const N: usize>(prefix: &[u8], keys: &impl WriteCompositeKey) -> CompositeKey<N> {
    let total_len = prefix.len() + keys.total_len();

    let mut composite_key = CompositeKey::new(total_len);

    composite_key.write_key_part(prefix);

    keys.write_into(&mut composite_key);

    composite_key
}

trait VisitBytes {
    fn visit_bytes<R, F: FnOnce(&[u8]) -> R>(&self, visitor: F) -> R;
}

impl<T> WriteCompositeKey for T
where
    T: VisitBytes,
{
    fn total_len(&self) -> usize {
        self.visit_bytes(<[u8]>::len)
    }

    fn write_into<W: WriteKeyPart>(&self, writer: &mut W) {
        self.visit_bytes(|bytes| writer.write_key_part(bytes));
    }
}

impl<T1, T2> WriteCompositeKey for (T1, T2)
where
    T1: VisitBytes,
    T2: VisitBytes,
{
    fn total_len(&self) -> usize {
        self.0.visit_bytes(<[u8]>::len) + self.1.visit_bytes(<[u8]>::len)
    }

    fn write_into<W: WriteKeyPart>(&self, writer: &mut W) {
        self.0.visit_bytes(|bytes| writer.write_key_part(bytes));
        self.1.visit_bytes(|bytes| writer.write_key_part(bytes));
    }
}

impl<T1, T2, T3> WriteCompositeKey for (T1, T2, T3)
where
    T1: VisitBytes,
    T2: VisitBytes,
    T3: VisitBytes,
{
    fn total_len(&self) -> usize {
        self.0.visit_bytes(<[u8]>::len)
            + self.1.visit_bytes(<[u8]>::len)
            + self.2.visit_bytes(<[u8]>::len)
    }

    fn write_into<W: WriteKeyPart>(&self, writer: &mut W) {
        self.0.visit_bytes(|bytes| writer.write_key_part(bytes));
        self.1.visit_bytes(|bytes| writer.write_key_part(bytes));
        self.2.visit_bytes(|bytes| writer.write_key_part(bytes));
    }
}

impl<'a> VisitBytes for &'a [u8] {
    fn visit_bytes<R, F: FnOnce(&[u8]) -> R>(&self, visitor: F) -> R {
        visitor(self)
    }
}

impl<'a> VisitBytes for &'a str {
    fn visit_bytes<R, F: FnOnce(&[u8]) -> R>(&self, visitor: F) -> R {
        visitor(self.as_bytes())
    }
}

impl VisitBytes for String {
    fn visit_bytes<R, F: FnOnce(&[u8]) -> R>(&self, visitor: F) -> R {
        visitor(self.as_bytes())
    }
}

impl VisitBytes for Vec<u8> {
    fn visit_bytes<R, F: FnOnce(&[u8]) -> R>(&self, visitor: F) -> R {
        visitor(self.as_slice())
    }
}

macro_rules! impl_visit_bytes_int {
    ($($t:ty),+) => {
        $(impl VisitBytes for $t {
            fn visit_bytes<R, F>(&self, visitor: F) -> R
            where
                F: FnOnce(&[u8]) -> R,
            {
                visitor(&self.to_be_bytes())
            }
        })*
    };
}

impl_visit_bytes_int!(u8, u16, u32, u64, u128);

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
