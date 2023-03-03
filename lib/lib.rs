#[cfg(not(any(feature = "serde", feature = "rkyv")))]
compile_error!(
    "You must choose the (de)serialization backend by specifying either 'serde' or 'rkyv' features"
);

#[cfg(all(feature = "serde", feature = "rkyv"))]
compile_error!(
    "You must choose only one (de)serialization backend feature. Either 'serde' or 'rkyv'";
);

#[cfg(feature = "serde")]
pub use serde;

#[cfg(feature = "serde")]
pub use kv_storage_serde::*;

#[cfg(feature = "rkyv")]
pub use rkyv;

#[cfg(feature = "rkyv")]
pub use kv_storage_rkyv::*;

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
