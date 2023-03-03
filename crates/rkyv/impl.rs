use std::{error::Error as StdError, marker::PhantomData};

use rkyv::{
    ser::Serializer as RkyvSerializer, Archive, Archived, Deserialize, Fallible, Infallible,
    Serialize,
};

pub trait Serializer {
    type Impl<'a>: Fallible + RkyvSerializer;
    type Error: StdError;

    fn serialize<'a, T>(&'a mut self, item: &T) -> Result<&'a [u8], Self::Error>
    where
        T: Serialize<Self::Impl<'a>>;
}

pub trait IntoOwned<T> {
    fn into_owned(self) -> T;
}

pub trait Deserializer {
    type Output<T>: AsRef<Archived<T>> + IntoOwned<T>
    where
        T: Archive,
        Archived<T>: Deserialize<T, Infallible>;

    type Error: StdError;

    fn deserialize<T>(bytes: Vec<u8>) -> Result<Self::Output<T>, Self::Error>
    where
        T: Archive,
        Archived<T>: Deserialize<T, Infallible>;
}

pub type Loaded<Store, T> = <<Store as Storage>::Serde as Deserializer>::Output<T>;

pub type MayLoadResult<Store, T, Error> = Result<Option<Loaded<Store, T>>, Error>;

pub type SerializerImpl<'a, Serde> = <Serde as Serializer>::Impl<'a>;

pub trait Storage {
    type Error: StdError;
    type Serde: Serializer + Deserializer;

    fn save<'a, T>(&'a mut self, key: &[u8], item: &T) -> Result<(), Self::Error>
    where
        T: Serialize<SerializerImpl<'a, Self::Serde>>;

    fn may_load<T>(&self, key: &[u8]) -> MayLoadResult<Self, T, Self::Error>
    where
        T: Archive,
        Archived<T>: Deserialize<T, Infallible>;
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

    pub fn save<'a, Store: Storage>(
        &self,
        store: &'a mut Store,
        item: &T,
    ) -> Result<(), Store::Error>
    where
        T: Serialize<SerializerImpl<'a, Store::Serde>>,
    {
        store.save(self.key, item)
    }

    pub fn may_load<Store: Storage>(&self, store: &Store) -> MayLoadResult<Store, T, Store::Error>
    where
        T: Archive,
        Archived<T>: Deserialize<T, Infallible>,
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

    pub fn save<'a, Store: Storage>(
        &self,
        store: &'a mut Store,
        key: &K,
        item: &V,
    ) -> Result<(), Store::Error>
    where
        V: Serialize<SerializerImpl<'a, Store::Serde>>,
    {
        let composite = [self.prefix, key.as_ref()].concat();
        store.save(&composite, item)
    }

    pub fn may_load<Store: Storage>(
        &self,
        store: &Store,
        key: &K,
    ) -> MayLoadResult<Store, V, Store::Error>
    where
        V: Archive,
        Archived<V>: Deserialize<V, Infallible>,
    {
        let composite = [self.prefix, key.as_ref()].concat();
        store.may_load::<V>(&composite)
    }
}

#[cfg(feature = "stock")]
mod stock_serde_impl {
    use std::marker::PhantomData;

    use rkyv::ser::serializers::{
        AlignedSerializer, AllocScratch, CompositeSerializer, FallbackScratch, HeapScratch,
        SharedSerializeMap,
    };
    use rkyv::{AlignedVec, Archive, Archived, Deserialize, Infallible, Serialize};

    use crate::{Deserializer, IntoOwned, Serializer};

    #[derive(Default)]
    pub struct StockSerde {
        buffer: AlignedVec,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("never going to error")]
    pub struct Never;

    type SerializerImpl<'a, const N: usize> = CompositeSerializer<
        AlignedSerializer<&'a mut AlignedVec>,
        FallbackScratch<HeapScratch<N>, AllocScratch>,
        SharedSerializeMap,
    >;

    fn make_serializer<S, C: Default, H: Default>(inner: S) -> CompositeSerializer<S, C, H> {
        CompositeSerializer::new(inner, C::default(), H::default())
    }

    impl Serializer for StockSerde {
        type Impl<'a> = SerializerImpl<'a, 1024>;
        type Error = Never;

        fn serialize<'a, T>(&'a mut self, item: &T) -> Result<&'a [u8], Self::Error>
        where
            T: Serialize<Self::Impl<'a>>,
        {
            use rkyv::ser::Serializer as _;

            let aligned_serializer = AlignedSerializer::new(&mut self.buffer);

            let mut composite_serializer = make_serializer(aligned_serializer);

            composite_serializer.serialize_value(item).unwrap();

            let buffer = composite_serializer.into_serializer().into_inner();

            Ok(buffer.as_slice())
        }
    }

    #[derive(Debug)]
    pub struct Deserialized<T> {
        buffer: Vec<u8>,
        _t: PhantomData<T>,
    }

    impl<T> IntoOwned<T> for Deserialized<T>
    where
        T: Archive,
        Archived<T>: Deserialize<T, Infallible>,
    {
        fn into_owned(self) -> T {
            // safe due to being used in a trusted context, i.e. only giving it buffers filled by Serializer
            let archived = unsafe { rkyv::archived_root::<T>(self.buffer.as_slice()) };
            let owned: T = archived.deserialize(&mut Infallible).unwrap();
            owned
        }
    }

    impl<T> AsRef<T::Archived> for Deserialized<T>
    where
        T: Archive,
    {
        fn as_ref(&self) -> &T::Archived {
            // safe due to being used in a trusted context, i.e. only giving it buffers filled by Serializer
            unsafe { rkyv::archived_root::<T>(self.buffer.as_slice()) }
        }
    }

    impl<T> std::ops::Deref for Deserialized<T>
    where
        T: Archive,
    {
        type Target = T::Archived;

        fn deref(&self) -> &Self::Target {
            self.as_ref()
        }
    }

    impl Deserializer for StockSerde {
        type Output<T> = Deserialized<T>
        where
            T: Archive,
            Archived<T>: Deserialize<T, Infallible>;
        type Error = Never;

        fn deserialize<T>(bytes: Vec<u8>) -> Result<Self::Output<T>, Self::Error>
        where
            T: Archive,
            Archived<T>: Deserialize<T, Infallible>,
        {
            Ok(Deserialized {
                buffer: bytes,
                _t: PhantomData,
            })
        }
    }
}

#[cfg(feature = "stock")]
pub use stock_serde_impl::*;
