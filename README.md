# Generic Key-Value Store (Experimental)

A set of traits to work with key-value stores where both the storage and (De)Serialization backends can be chosen by upstream consumers.

Currently there are two competing implementations, one based on `serde` traits and another on `rkyv`'s (zero-copy deserialization).

Benchmarks coming soon™.

Check `test/mock/consumer.rs` for an example of how the backends affect upstream APIs.

## Testing

```
// Serde impl
$ cargo test-serde

// Rkyv impl
$ cargo test-rkyv
```
