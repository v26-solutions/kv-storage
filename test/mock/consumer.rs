use kv_storage::{item, map, Item, Map, MutStorage, Storage};

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
    total: u128,
    balance: u128,
}

#[must_use]
pub struct Modified<T>(T);

pub type ModifiedBalance<'a> = Modified<&'a mut Balance<'a>>;

impl<'a> Balance<'a> {
    pub fn balance(&self) -> u128 {
        self.balance
    }

    pub fn total(&self) -> u128 {
        self.total
    }

    pub fn withdraw(&'a mut self, amount: u128) -> Result<ModifiedBalance<'a>, Error> {
        if amount > self.balance {
            return Err(Error::InsufficientFunds);
        }

        self.balance -= amount;
        self.total -= amount;

        Ok(Modified(self))
    }

    pub fn deposit(&'a mut self, amount: u128) -> Result<ModifiedBalance<'a>, Error> {
        let (balance, overflow) = self.balance.overflowing_add(amount);

        if overflow {
            return Err(Error::BalanceOverflow);
        }

        let (total, overflow) = self.total.overflowing_add(amount);

        if overflow {
            return Err(Error::BalanceOverflow);
        }

        self.balance = balance;
        self.total = total;

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

impl<'a> Balance<'a> {
    const BALANCES: Map<1024, &str, u128> = map!("balances");
    const TOTAL: Item<u128> = item!("total_balance");

    fn save<Store: MutStorage>(&self, store: &mut Store) -> Result<(), Error<Store::Error>> {
        Self::TOTAL.save(store, &self.total)?;

        Self::BALANCES
            .save(store, &self.account, &self.balance)
            .map_err(Error::from)
    }

    pub fn account_exists<Store: Storage>(
        store: &Store,
        account: &'a str,
    ) -> Result<bool, Error<Store::Error>> {
        Self::BALANCES.has_key(store, &account).map_err(Error::from)
    }

    pub fn load_total<Store: Storage>(store: &Store) -> Result<u128, Error<Store::Error>> {
        let total = Self::TOTAL.may_load(store)?.unwrap_or_default();
        Ok(total)
    }

    pub fn load_account<Store: Storage>(
        store: &Store,
        account: &'a str,
    ) -> Result<Balance<'a>, Error<Store::Error>> {
        let maybe_balance = Self::BALANCES.may_load(store, &account)?;
        let maybe_total = Self::TOTAL.may_load(store)?;

        Ok(Balance {
            account,
            total: maybe_total.unwrap_or_default(),
            balance: maybe_balance.unwrap_or_default(),
        })
    }
}

impl<'a> ModifiedBalance<'a> {
    pub fn save<Store: MutStorage>(
        self,
        store: &mut Store,
    ) -> Result<&'a mut Balance<'a>, Error<Store::Error>> {
        self.0.save(store)?;
        Ok(self.0)
    }
}
