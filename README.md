# Rust-mqtt

The `rust-mqtt` crate is a native async MQTT client for both std and no_std environments.
The client library provides an async API which can be used with various executors.

## MQTT Standard

### Supported features

- QoS 0 & QoS 1
- Basic Authentication via Connect
- Subscription Options (Retain Handling, Retain as Published & No Local)

### Missing features

- Quality of Service Level 2
- Non-clean session
- Auth packet

## Implementation & Usage details

- Packet size is not limited, it is totally up to user (packet size and buffer sizes have to align)

## Tests

Integration tests are written using tokio network tcp stack.

For local testing, using Mosquitto is recommended. Run the following commands once to set up Mosquitto

```bash
cp .ci/mqtt_pass_plain.txt .ci/mqtt_pass_hashed.txt
chmod 700 .ci/mqtt_pass_hashed.txt
mosquitto_passwd -U .ci/mqtt_pass_hashed.txt
```

Start Mosquitto

```bash
mosquitto -c .ci/mosquitto.conf
```

The test categories can be executed separately using these arguments

```bash
cargo test unit
cargo test integration
cargo test load
```

Individual tests can be run as follows

```bash
RUST_LOG=info cargo test -- integration::tests::pubsub::publish_recv --exact
```

## Minimum supported Rust version (MSRV)

Rust-mqtt is guaranteed to compile on stable Rust 1.86 and up.
It might compile with older versions but that may change in any new patch release.

## Acknowledgment

This project could not be in state in which currently is without Ulf Lilleengen and rest of the community
from [Drogue IoT](https://github.com/drogue-iot).

## Contact

For any information contact me on email <ond.babec@gmail.com>
