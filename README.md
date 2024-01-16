# Rust-mqtt
## About
Rust-mqtt is native MQTT client for both std and no_std environments.
Client library provides async API which can be used with various executors.
Currently, supporting only MQTTv5 but everything is prepared to extend support also
for MQTTv3 which is planned during year 2022.

## Async executors
For desktop usage I recommend using Tokio async executor and for embedded there is prepared wrapper for Drogue device
framework in the Drogue-IoT project [examples](https://github.com/drogue-iot/drogue-device/tree/main/device/src/network/clients) mqtt module.

## Restrains
Client supports following:
- QoS 0 & QoS 1 (All QoS 2 packets are mapped for future client extension)
- Only clean session
- Retain not supported
- Auth packet not supported
- Packet size is not limited, it is totally up to user (packet size and buffer sizes have to align)

## Building
```
cargo build
```

## Running tests
Integration tests are written using tokio network tcp stack and can be find under tokio_net.
```
cargo test unit
cargo test integration
cargo test load
```

## Minimum supported Rust version (MSRV)
Rust-mqtt is guaranteed to compile on stable Rust 1.75 and up.
It might compile with older versions but that may change in any new patch release.

## Acknowledgment
This project could not be in state in which currently is without Ulf Lilleengen and rest of the community
from [Drogue IoT](https://github.com/drogue-iot).

## Contact
For any information contact me on email <ond.babec@gmail.com>
