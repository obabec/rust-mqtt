# rust-mqtt &emsp; [![build]][actions] [![docs]][docs.rs] [![crates]][crates.io] [![msrv]][rust 1.87] [![license]][MIT]

[build]: https://img.shields.io/github/actions/workflow/status/obabec/rust-mqtt/ci.yaml?branch=main&label=ci
[actions]: https://github.com/obabec/rust-mqtt/actions?query=branch%3Amain
[docs]: https://docs.rs/rust-mqtt/badge.svg
[docs.rs]: https://docs.rs/rust-mqtt
[crates]: https://img.shields.io/crates/v/rust-mqtt.svg
[crates.io]: https://crates.io/crates/rust-mqtt
[msrv]: https://img.shields.io/crates/msrv/rust-mqtt.svg?color=lightgray
[rust 1.87]: https://blog.rust-lang.org/2025/05/15/Rust-1.87.0/
[license]: https://img.shields.io/crates/l/rust-mqtt.svg
[MIT]: https://github.com/obabec/rust-mqtt#license

`rust-mqtt` provides a MQTT client primarily for no_std environments. The library provides an async API depending on [embedded_io_async](https://docs.rs/embedded-io-async/latest/embedded_io_async/)'s traits. As of now, only [MQTT version 5.0](https://docs.oasis-open.org/mqtt/mqtt/v5.0/mqtt-v5.0.html) is supported.

The design goal is a strict yet flexible and explicit API that leverages Rust's type system to enforce the MQTT specification while exposing all protocol features transparently. Session state, configuration, and Quality of Service message delivery and retry behaviour remain fully under user control, giving complete freedom over protocol usage. Protocol-related errors are prevented by the client API and are modeled in a way that enables maximum recoverability. By avoiding opinionated design choices and making no assumptions about the runtime environment, `rust-mqtt` remains lightweight while providing a powerful MQTT client foundation.

`rust-mqtt` does not implement opinionated connection management — automatic reconnects, keepalive loops, retry policies, or background tasks are intentionally left to the user. Instead, the crate is designed as a composable protocol layer, suitable for either higher-level clients, tooling or resource-constrained embedded applications.

## Library state

### Supported MQTT features

- Will
- Bidirectional publications with Quality of Service 0, 1 and 2
- Flow control
- Configuration & session tracking
- Session recovery
- Serverside maximum packet size
- Subscription identifiers
- Message expiry interval
- Topic alias
- Request/Response

### Currently unsupported MQTT features & limitations

- AUTH packet
- User properties
- Subscribing to multiple topics in a single packet

### Extension plans (more or less by priority)

- Read complete packets with cancel-safe implementation
- MQTT version 3.1.1
- Sync implementation

### Feature flags

- `log`: Enables logging via the `log` crate
- `defmt`: Implements `defmt::Format` for crate items & enables logging via the `defmt` crate (version 1)
- `bump`: Adds a simple bump allocator `BufferProvider` implementation
- `alloc`: Adds an `Owned(Box<[u8]>)` variant to `Bytes` and a heap-allocation based `BufferProvider` implementation using the `alloc` crate
- `v3`: Unused
- `v5`: Enables MQTT version 5.0

## Usage

### Examples

- 'demo' is a showcase of rust-mqtt's features over TCP. Note that the example usage is very strict and not really a good way of using the client.
- 'tls' connects the client to a broker over TLS with client certificate authentication and server certificate verification using [embedded-tls](https://github.com/drogue-iot/embedded-tls).

Set up the broker for 'demo' by installing, configuring and running Mosquitto using the CI configuration:

```bash
cp .ci/mqtt_pass_plain.txt .ci/mqtt_pass_hashed.txt
chmod 700 .ci/mqtt_pass_hashed.txt
mosquitto_passwd -U .ci/mqtt_pass_hashed.txt
mosquitto -c .ci/mosquitto.conf -v
```

Set up the broker for 'tls' by running Mosquitto with the tls config file. The required PKI files have been generated using the `.ci/pki/generate.sh` script.

```bash
mosquitto -c .ci/mosquitto-tls.conf -v
```

Then you can run the examples with different logging configs and the bump/alloc features:

```bash
RUST_LOG=debug cargo run --example demo
RUST_LOG=info cargo run --example tls
RUST_LOG=trace cargo run --example demo --no-default-features --features "v5 log bump"
```

### Tests

Unit tests should be ran using both the 'alloc' and 'bump' features.

```bash
cargo test unit
cargo test unit --no-default-features --features "v5 bump"
```

For integration tests, you can set up the mosquitto broker as used in the CI pipeline.
You should restart the broker after every run of the integration test suite as it
carries non-idempotent state that will impact the tests.

```bash
cp .ci/mqtt_pass_plain.txt .ci/mqtt_pass_hashed.txt
chmod 700 .ci/mqtt_pass_hashed.txt
mosquitto_passwd -U .ci/mqtt_pass_hashed.txt
mosquitto -c .ci/mosquitto.conf [-d]
```

Then you can run integration tests with the alloc feature.

```bash
cargo test integration
```

It can be helpful to see logging output when running tests.

```bash
RUST_LOG=trace cargo test unit --no-default-features --features "v5 bump" -- --show-output
RUST_LOG=warn cargo test -- --show-output
RUST_LOG=info cargo test integration -- --show-output
```

The full test suite can run with the alloc feature, just make sure a fresh broker is up and running.

```bash
cargo test
```

## Acknowledgment

This project could not be in state in which currently is without Ulf Lilleengen and the rest of the community
from [Drogue IoT](https://github.com/drogue-iot).

## Contact

For any information, open an issue if your matter could be helpful or interesting for others or should be documented. Otherwise contact us on email <julian.jg.graf@gmail.com>, <ond.babec@gmail.com>.
