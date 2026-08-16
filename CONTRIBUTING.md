# Contributing to rust-mqtt

`rust-mqtt` welcomes contribution from everyone in the form of suggestions, bug reports, pull requests, and feedback. This document gives some guidance if you are thinking of helping us or just want to run the examples.

## Setting up a broker

For our examples and integration test suite, a MQTT broker is required. For local development, Mosquitto is recommended over HiveMQ because it's simpler to set up.

To configure and run a broker for plain TCP connections (`demo`, `manual_ack`, integration tests) run the following commands:

```bash
cp .ci/mqtt_pass_plain.txt .ci/mqtt_pass_hashed.txt
chmod 700 .ci/mqtt_pass_hashed.txt
mosquitto_passwd -U .ci/mqtt_pass_hashed.txt
mosquitto -c .ci/mosquitto.conf -v
```

Set up the broker for `tls` by running Mosquitto with the TLS configuration file. The required PKI files have been generated using `.ci/pki/generate.sh` script.

```bash
mosquitto -c .ci/mosquitto-tls.conf -v
```

## Examples

- 'demo' is a showcase of rust-mqtt's features over TCP. Note that the example usage is very strict and not really a good way of using the client.
- 'tls' connects the client to a broker over TLS with client certificate authentication and server certificate verification using [embedded-tls](https://github.com/drogue-iot/embedded-tls).
- 'manual_ack' shows the client's capabilities of manual acknowledgements by rudimentarily implementing the optional payload format check (see [MQTTv5, 3.3.2.3.2 Payload Format Indicator](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901111))

Set up the broker for 'demo' and 'manual_ack' by installing, configuring and running Mosquitto using the CI configuration:

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
RUST_LOG=info cargo run --example demo
RUST_LOG=info cargo run --example tls
RUST_LOG=trace cargo run --example manual_ack --no-default-features --features "v5 log bump log-level-trace"
```

## Tests

The CI pipeline run unit, integration and doc tests as well as linting and other checks thoroughly. However, you should still run these tests locally.

### Unit

Unit tests should be ran using both the `alloc` and `bump` features.

```bash
cargo test unit
cargo test unit --no-default-features --features "v5 bump"
```

### Integration

Set up the mosquitto broker as used in the CI pipeline (described above). You should restart the broker after every run of the integration test suite as it carries non-idempotent state that will impact the tests. Integration tests can only be run with the `alloc` feature for simplicity.

```bash
cargo test integration
```

Because the test suite is quite comprehensive, some test cases are ignored because they may fail with the currently used release of a broker. However, these cases also follow a naming convention to be able to run them despite being ignored by default. To run these tests on the relevant broker that does behave correctly, you can run the following commands.

```bash
cargo test mosquitto_only -- --ignored
cargo test hive_only -- --ignored
```

### Debugging

It can be helpful to see logging output when running tests. `rust-mqtt` supports detailed logging. At `log-level-trace`, the state machine logs all state transitions and at `log-verbose`, I/O traces when encoding and decoding the packets are also enabled.

```bash
RUST_LOG=trace cargo test unit --no-default-features --features "v5 bump log-verbose log" -- --show-output
RUST_LOG=warn cargo test -- --show-output
RUST_LOG=info cargo test integration --features "log" -- --show-output
```
