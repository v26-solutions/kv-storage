#[derive(Debug, thiserror::Error)]
pub enum Error<S = ()> {
    #[error(transparent)]
    Storage(#[from] S),
    #[error("item not found")]
    NotFound,
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("balance overflow")]
    BalanceOverflow,
}

pub struct Balance<'a> {
    account: &'a str,
    balance: u128,
}

#[must_use]
pub struct Modified<T>(T);

pub type ModifiedBalance<'a> = Modified<&'a mut Balance<'a>>;

impl<'a> Balance<'a> {
    pub fn balance(&self) -> u128 {
        self.balance
    }

    pub fn withdraw(&'a mut self, amount: u128) -> Result<ModifiedBalance<'a>, Error> {
        if amount > self.balance {
            return Err(Error::InsufficientFunds);
        }

        self.balance -= amount;

        Ok(Modified(self))
    }

    pub fn deposit(&'a mut self, amount: u128) -> Result<ModifiedBalance<'a>, Error> {
        let (balance, overflow) = self.balance.overflowing_add(amount);

        if overflow {
            return Err(Error::BalanceOverflow);
        }

        self.balance = balance;

        Ok(Modified(self))
    }
}

impl<'a> std::ops::Deref for ModifiedBalance<'a> {
    type Target = Balance<'a>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> std::ops::DerefMut for ModifiedBalance<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

#[cfg(any(feature = "serde", feature = "rkyv"))]
mod balance_storage_impl {
    use kv_storage::{map, Map, Storage};

    use crate::{Balance, Error, ModifiedBalance};

    impl<'a> Balance<'a> {
        const MAP: Map<&str, u128> = map!("balances");

        fn save<Store: Storage>(&self, store: &mut Store) -> Result<(), Error<Store::Error>> {
            Self::MAP
                .save(store, &self.account, &self.balance)
                .map_err(Error::from)
        }

        pub fn load<Store: Storage>(
            store: &Store,
            account: &'a str,
        ) -> Result<Balance<'a>, Error<Store::Error>> {
            let maybe_loaded = Self::MAP.may_load(store, &account)?;

            Ok(Balance {
                account,
                #[cfg(feature = "serde")]
                balance: maybe_loaded.unwrap_or_default(),
                #[cfg(feature = "rkyv")]
                balance: maybe_loaded.map_or(0, |v| *v.as_ref()),
            })
        }
    }

    impl<'a> ModifiedBalance<'a> {
        pub fn save<Store: Storage>(
            self,
            store: &mut Store,
        ) -> Result<&'a mut Balance<'a>, Error<Store::Error>> {
            self.0.save(store)?;
            Ok(self.0)
        }
    }
}

#[cfg(feature = "serde")]
mod config {
    use kv_storage::serde::{Deserialize, Serialize};
    use kv_storage::{item, Item, Storage};

    use crate::Error;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(crate = "::kv_storage::serde")]
    pub struct Config {
        pub foo: String,
        pub bar: String,
        pub baz: u128,
    }

    impl Config {
        const SLOT: Item<Self> = item!("config");

        pub fn save<Store: Storage>(&self, store: &mut Store) -> Result<(), Error<Store::Error>> {
            Self::SLOT.save(store, self).map_err(Error::from)
        }

        pub fn load<Store: Storage>(store: &Store) -> Result<Self, Error<Store::Error>> {
            Self::SLOT
                .may_load(store)
                .map_err(Error::from)
                .and_then(|found| found.ok_or(Error::NotFound))
        }

        pub fn summarize(&self) -> String {
            format!("foo:{}:bar:{}:baz:{}", self.foo, self.bar, self.baz)
        }
    }
}

#[cfg(feature = "rkyv")]
mod config {
    use kv_storage::rkyv::{Archive, Archived, Deserialize, Serialize};
    use kv_storage::{item, Item, Loaded, Storage};

    use crate::Error;

    #[derive(Clone, Debug, Archive, Serialize, Deserialize)]
    #[archive(crate = "::kv_storage::rkyv")]
    pub struct Config {
        pub foo: String,
        pub bar: String,
        pub baz: u128,
    }

    pub trait ConfigExt {
        fn summarize(&self) -> String;
    }

    impl Config {
        const SLOT: Item<Self> = item!("config");

        pub fn save<Store: Storage>(&self, store: &mut Store) -> Result<(), Error<Store::Error>> {
            Self::SLOT.save(store, self).map_err(Error::from)
        }

        pub fn load<Store: Storage>(
            store: &Store,
        ) -> Result<Loaded<Store, Self>, Error<Store::Error>> {
            Self::SLOT
                .may_load(store)
                .map_err(Error::from)
                .and_then(|found| found.ok_or(Error::NotFound))
        }
    }

    impl ConfigExt for Archived<Config> {
        fn summarize(&self) -> String {
            format!("foo:{}:bar:{}:baz:{}", self.foo, self.bar, self.baz)
        }
    }
}

#[cfg(any(feature = "serde", feature = "rkyv"))]
pub use config::*;
