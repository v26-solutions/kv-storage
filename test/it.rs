#[cfg(test)]
mod test {
    use kv_storage::KvStore;
    use kv_storage_bincode::Bincode;
    use kv_storage_memory::MemoryRepo;

    use mock_consumer::Balance;

    #[test]
    fn it_works() {
        let mut storage: KvStore<Bincode, MemoryRepo> = KvStore::default();

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
