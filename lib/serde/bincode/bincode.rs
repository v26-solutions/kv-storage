use kv_storage::{Deserializer, Fallible, Serializer};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "bincode-no-custom")]
use bincode_no_custom as bincode;

#[derive(Default)]
pub struct Bincode {
    buffer: Vec<u8>,
}

pub type Error = bincode::Error;

impl Bincode {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_buffer(buffer: Vec<u8>) -> Self {
        Self { buffer }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }
}

impl Fallible for Bincode {
    type Error = Error;
}

impl Serializer for Bincode {
    fn serialize<T: Serialize>(&mut self, item: &T) -> Result<&[u8], Self::Error> {
        self.buffer.clear();
        bincode::serialize_into(&mut self.buffer, item)?;
        Ok(&self.buffer)
    }
}

impl Deserializer for Bincode {
    fn deserialize<T: DeserializeOwned>(bytes: Vec<u8>) -> Result<T, Self::Error> {
        bincode::deserialize(&bytes)
    }
}
