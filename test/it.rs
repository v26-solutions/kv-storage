#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use kv_storage::{Deserializer, Fallible, GenericStorage, HasKey, Read, Serializer, Write};

    use serde::{de::DeserializeOwned, Serialize};

    use mock_consumer::Balance;

    #[derive(Debug, thiserror::Error)]
    #[error("infallible")]
    struct Infallible;

    #[derive(Default)]
    struct MemRepo {
        map: HashMap<Vec<u8>, Vec<u8>>,
    }

    impl Fallible for MemRepo {
        type Error = Infallible;
    }

    impl Write for MemRepo {
        fn write(&mut self, key: &[u8], bytes: &[u8]) -> Result<(), Self::Error> {
            self.map.insert(key.to_owned(), bytes.to_owned());
            Ok(())
        }
    }

    impl Read for MemRepo {
        fn read(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
            Ok(self.map.get(key).map(Clone::clone))
        }
    }

    impl HasKey for MemRepo {
        fn has_key(&self, key: &[u8]) -> Result<bool, Self::Error> {
            Ok(self.map.contains_key(key))
        }
    }

    #[derive(Default)]
    struct BinSerde {
        buffer: Vec<u8>,
    }

    impl Fallible for BinSerde {
        type Error = bincode::Error;
    }

    impl Serializer for BinSerde {
        fn serialize<T: Serialize>(&mut self, item: &T) -> Result<&[u8], Self::Error> {
            bincode::serialize_into(&mut self.buffer, item)?;
            Ok(&self.buffer)
        }
    }

    impl Deserializer for BinSerde {
        fn deserialize<T: DeserializeOwned>(bytes: Vec<u8>) -> Result<T, Self::Error> {
            bincode::deserialize(&bytes)
        }
    }

    #[test]
    fn it_works() {
        let mut storage: GenericStorage<BinSerde, MemRepo> = GenericStorage::default();

        assert!(!Balance::account_exists(&storage, "alice").unwrap());

        let mut alice = Balance::load_account(&storage, "alice").unwrap();

        assert_eq!(alice.balance(), 0);
        assert_eq!(alice.total(), 0);

        alice
            .deposit(1000)
            .unwrap()
            .withdraw(500)
            .unwrap()
            .save(&mut storage)
            .unwrap();

        assert_eq!(
            Balance::load_account(&storage, "alice").unwrap().balance(),
            500
        );

        assert_eq!(Balance::load_total(&storage).unwrap(), 500);
    }
}
