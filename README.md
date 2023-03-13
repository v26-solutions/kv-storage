# Generic Key-Value Store (Experimental)

A set of traits to work with key-value stores where both the storage and (De)Serialization backends can be chosen by upstream consumers.

Only `serde` base `Serialize` & `Deserialize` traits supported for now.

## Usage

See `test/mock/consumer.rs`

## Testing

```
$ cargo t
```
