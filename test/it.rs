#[cfg(test)]
mod memstore {
    use std::collections::HashMap;

    #[cfg(any(feature = "serde", feature = "rkyv"))]
    use kv_storage::{Deserializer, Serializer, Storage};

    #[cfg(feature = "serde")]
    use kv_storage::serde::{de::DeserializeOwned, Serialize};

    #[cfg(feature = "rkyv")]
    use kv_storage::rkyv::{Archive, Archived, Deserialize, Infallible, Serialize};

    #[cfg(feature = "rkyv")]
    use kv_storage::{MayLoadResult, SerializerImpl};

    #[derive(Debug, thiserror::Error)]
    pub enum Error<D, S> {
        #[error(transparent)]
        Deserialize(D),
        #[error(transparent)]
        Serialize(S),
    }

    pub struct MemStore<Serde> {
        map: HashMap<Vec<u8>, Vec<u8>>,
        serializer: Serde,
    }

    #[cfg(any(feature = "serde", feature = "rkyv"))]
    impl<Serde> MemStore<Serde>
    where
        Serde: Deserializer + Serializer,
    {
        pub fn new(serializer: Serde) -> MemStore<Serde> {
            MemStore {
                map: HashMap::new(),
                serializer,
            }
        }
    }

    #[cfg(any(feature = "serde", feature = "rkyv"))]
    type ErrorWith<Serde> = Error<<Serde as Deserializer>::Error, <Serde as Serializer>::Error>;

    #[cfg(feature = "serde")]
    impl<Serde> Storage for MemStore<Serde>
    where
        Serde: Serializer + Deserializer,
    {
        type Error = ErrorWith<Serde>;
        type Serde = Serde;

        fn save<T>(&mut self, key: &[u8], item: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            let bytes = self
                .serializer
                .serialize(item)
                .map_err(Error::Serialize)?
                .as_ref()
                .to_owned();

            self.map.insert(key.to_owned(), bytes);

            Ok(())
        }

        fn may_load<T>(&self, key: &[u8]) -> Result<Option<T>, Self::Error>
        where
            T: DeserializeOwned,
        {
            let Some(bytes) = self.map.get(key) else {
                return Ok(None);
            };

            Serde::deserialize(bytes.clone())
                .map(Some)
                .map_err(Error::Deserialize)
        }
    }

    #[cfg(feature = "rkyv")]
    impl<Serde> Storage for MemStore<Serde>
    where
        Serde: Serializer + Deserializer,
    {
        type Error = ErrorWith<Serde>;
        type Serde = Serde;

        fn save<'a, T>(&'a mut self, key: &[u8], item: &T) -> Result<(), Self::Error>
        where
            T: Serialize<SerializerImpl<'a, Self::Serde>>,
        {
            let bytes = self
                .serializer
                .serialize(item)
                .map_err(Error::Serialize)?
                .as_ref()
                .to_owned();

            self.map.insert(key.to_owned(), bytes);

            Ok(())
        }

        fn may_load<T>(&self, key: &[u8]) -> MayLoadResult<Self, T, Self::Error>
        where
            T: Archive,
            Archived<T>: Deserialize<T, Infallible>,
        {
            let Some(bytes) = self.map.get(key) else {
                return Ok(None);
            };

            Serde::deserialize(bytes.clone())
                .map(Some)
                .map_err(Error::Deserialize)
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde {
    use kv_storage::serde::{de::DeserializeOwned, Serialize};
    use kv_storage::{Deserializer, Serializer};

    #[derive(Default)]
    pub struct SerdeJson {
        scratch: Vec<u8>,
    }

    impl Serializer for SerdeJson {
        type Error = serde_json::Error;

        fn serialize<'a, T: Serialize>(&'a mut self, item: &T) -> Result<&'a [u8], Self::Error> {
            self.scratch.clear();

            serde_json::to_writer(&mut self.scratch, item)?;

            Ok(self.scratch.as_slice())
        }
    }

    impl Deserializer for SerdeJson {
        type Error = serde_json::Error;

        fn deserialize<T: DeserializeOwned>(bytes: Vec<u8>) -> Result<T, Self::Error> {
            serde_json::from_slice(&bytes)
        }
    }
}

#[cfg(all(test, feature = "serde"))]
use serde::SerdeJson as SerdeImpl;

#[cfg(all(test, feature = "rkyv"))]
use kv_storage::StockSerde as SerdeImpl;

#[cfg(test)]
mod test {
    use mock_consumer::{Balance, Config, Error};

    #[cfg(feature = "rkyv")]
    use mock_consumer::ConfigExt;

    use crate::memstore::MemStore;
    use crate::SerdeImpl;

    #[test]
    fn it_works() {
        let mut storage = MemStore::new(SerdeImpl::default());

        assert!(matches!(
            Config::load(&storage).unwrap_err(),
            Error::NotFound
        ));

        let config = Config {
            foo: "foo".to_owned(),
            bar: "bar".to_owned(),
            baz: 123456,
        };

        config.save(&mut storage).unwrap();

        assert!(matches!(
            Config::load(&storage).unwrap().summarize().as_str(),
            "foo:foo:bar:bar:baz:123456"
        ));

        let mut alice = Balance::load(&storage, "alice").unwrap();

        assert_eq!(alice.balance(), 0);

        alice
            .deposit(1000)
            .unwrap()
            .withdraw(500)
            .unwrap()
            .save(&mut storage)
            .unwrap();

        assert_eq!(Balance::load(&storage, "alice").unwrap().balance(), 500);
    }
}
