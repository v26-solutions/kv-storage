use kv_storage::{Fallible, HasKey, Read, Remove, Write};

use cosmwasm_std::Storage;

pub struct CosmwasmRepo<T>(T);

impl<T> CosmwasmRepo<T> {
    pub const fn readonly(storage: &dyn Storage) -> CosmwasmRepo<&dyn Storage> {
        CosmwasmRepo(storage)
    }

    pub fn mutable(storage: &mut dyn Storage) -> CosmwasmRepo<&mut dyn Storage> {
        CosmwasmRepo(storage)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("infallible")]
pub struct Infallible;

impl<T> Fallible for CosmwasmRepo<T> {
    type Error = Infallible;
}

impl<'a> Write for CosmwasmRepo<&'a mut dyn Storage> {
    fn write(&mut self, key: &[u8], bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.set(key, bytes);
        Ok(())
    }
}

impl<'a> Read for CosmwasmRepo<&'a dyn Storage> {
    fn read(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.0.get(key))
    }
}

impl<'a> HasKey for CosmwasmRepo<&'a dyn Storage> {
    fn has_key(&self, key: &[u8]) -> Result<bool, Self::Error> {
        Ok(self.0.get(key).is_some())
    }
}

impl<'a> Remove for CosmwasmRepo<&'a mut dyn Storage> {
    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Error> {
        self.0.remove(key);
        Ok(())
    }
}
