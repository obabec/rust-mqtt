# rust-mqtt

rust-mqtt is an MQTT client primarily for no_std environments. The library provides an async API depending on [embedded_io_async](https://docs.rs/embedded-io-async/latest/embedded_io_async/)'s traits. As of now, only [MQTT version 5.0](https://docs.oasis-open.org/mqtt/mqtt/v5.0/mqtt-v5.0.html) is supported.

## Library state

### Supported MQTT features

- Will
- Bidirectional publications with Quality of Service 0, 1 and 2
- Flow control
- Configuration & session tracking
- Session recovery
- Serverside maximum packet size

### Currently unsupported MQTT features & limitations

- AUTH packet
- User properties
- Request/Response
- Subscription identifiers
- Topic alias
- Subscribing to multiple topics in a single packet
- Message expiry interval

### Extension plans (more or less by priority)

- Receive the 'remaining length' (variable header & payload) of an mqtt packet using a buffer instead of calling `Read::read` very frequently.
- MQTT version 3
- Sync implementation.

### Feature flags

- `log`: Enables logging via the `log` crate
- `defmt`: Implements `defmt::Format` for crate items & enables logging via the `defmt` crate (version 1)
- `bump`: Adds a simple bump allocator `BufferProvider` implementation
- `alloc`: Adds a heap-allocation based `BufferProvider` implementation using the `alloc` crate
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
