#[cfg(test)]
mod test {
    use kv_storage::{map, KvStore, Map};
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

    #[test]
    fn composite_keys_work() {
        const DOUBLE: Map<1024, (&str, &str), String> = map!("double_key");
        const TRIPLE: Map<1024, (&str, &str, &str), String> = map!("triple_key");

        let mut storage: KvStore<Bincode, MemoryRepo> = KvStore::default();

        DOUBLE
            .save(&mut storage, ("alice", "bob"), "hello".to_owned())
            .unwrap();

        TRIPLE
            .save(&mut storage, ("alice", "bob", "eve"), "hello".to_owned())
            .unwrap();

        assert_eq!(
            DOUBLE
                .may_load(&storage, ("alice", "bob"))
                .unwrap()
                .unwrap(),
            "hello"
        );

        assert_eq!(
            TRIPLE
                .may_load(&storage, ("alice", "bob", "eve"))
                .unwrap()
                .unwrap(),
            "hello"
        );
    }
}
