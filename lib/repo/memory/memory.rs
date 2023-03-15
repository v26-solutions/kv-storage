use std::collections::HashMap;

use kv_storage::{Fallible, HasKey, Read, Remove, Write};

#[derive(Debug, thiserror::Error)]
#[error("infallible")]
pub struct Infallible;

#[derive(Default)]
pub struct MemoryRepo {
    map: HashMap<Vec<u8>, Vec<u8>>,
}

impl Fallible for MemoryRepo {
    type Error = Infallible;
}

impl Write for MemoryRepo {
    fn write(&mut self, key: &[u8], bytes: &[u8]) -> Result<(), Self::Error> {
        self.map.insert(key.to_owned(), bytes.to_owned());
        Ok(())
    }
}

impl Read for MemoryRepo {
    fn read(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.map.get(key).map(Clone::clone))
    }
}

impl HasKey for MemoryRepo {
    fn has_key(&self, key: &[u8]) -> Result<bool, Self::Error> {
        Ok(self.map.contains_key(key))
    }
}

impl Remove for MemoryRepo {
    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Error> {
        self.map.remove(key);
        Ok(())
    }
}
