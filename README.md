# rust-mqtt &emsp; [![build]][actions] [![docs]][docs.rs] [![crates]][crates.io] [![msrv]][rust 1.87] [![license]][MIT OR APACHE-2.0]

[build]: https://img.shields.io/github/actions/workflow/status/obabec/rust-mqtt/ci.yaml?branch=main&label=ci
[actions]: https://github.com/obabec/rust-mqtt/actions?query=branch%3Amain
[docs]: https://docs.rs/rust-mqtt/badge.svg
[docs.rs]: https://docs.rs/rust-mqtt
[crates]: https://img.shields.io/crates/v/rust-mqtt.svg
[crates.io]: https://crates.io/crates/rust-mqtt
[msrv]: https://img.shields.io/crates/msrv/rust-mqtt.svg?color=lightgray
[rust 1.87]: https://blog.rust-lang.org/2025/05/15/Rust-1.87.0/
[license]: https://img.shields.io/crates/l/rust-mqtt.svg
[MIT OR APACHE-2.0]: https://github.com/obabec/rust-mqtt#license

`rust-mqtt` provides an MQTT client primarily for `no_std` environments. The library provides an async API depending on [embedded_io_async](https://docs.rs/embedded-io-async/latest/embedded_io_async/)'s traits. As of now, only [MQTT version 5.0](https://docs.oasis-open.org/mqtt/mqtt/v5.0/mqtt-v5.0.html) is supported.

The design goal is a strict yet flexible and explicit API that leverages Rust's type system to enforce the MQTT specification while exposing all protocol features transparently. Session state, configuration, and Quality of Service message delivery and retry behaviour remain fully under user control, giving complete freedom over protocol usage. Protocol-related errors are prevented by the client API and are modeled in a way that enables maximum recoverability. By avoiding opinionated design choices and making no assumptions about the runtime environment, `rust-mqtt` remains lightweight while providing a powerful MQTT client foundation.

`rust-mqtt` does not implement opinionated connection management — automatic reconnects, keepalive loops, retry policies, or background tasks are intentionally left to the user. Instead, the crate provides cancel-safe protocol primitives and optional manual acknowledgements, suitable for higher-level clients, tooling, and resource-constrained embedded applications. In the future, the client will be extended with additional I/O traits such as `ReadReady` to further composability.

## Library state

### Supported MQTT features

- Will
- Bidirectional publications with Quality of Service 0, 1 and 2
- Automatic and manual/deferred acknowledgements
- Flow control
- Configuration & session tracking
- Session recovery
- Client- & server-side maximum packet size
- Subscription identifiers
- Shared & wildcard subscriptions
- Message expiry interval
- Topic alias in outgoing publications
- Request/Response
- Request Problem Information
- Reason String
- User Property

### Currently unsupported MQTT features & limitations

- AUTH packet
- Properties: Authentication Method, Authentication Data
- Subscribing to multiple topics in a single packet
- Topic alias in incoming publications

### Extension plans (more or less by priority)

- More versatile IO model allowing for more cancel-safety
- Sync implementation
- MQTT version 3.1.1

### Feature flags

- `bump`: Adds a simple bump allocator `BufferProvider` implementation
- `alloc`: Adds an `Owned(Box<[u8]>)` variant to `Bytes` and a heap-allocation based `BufferProvider` implementation using the `alloc` crate
- `v3`: Unused
- `v5`: Enables MQTT version 5.0
- Logging-related:
  - `log`: Enables logging via the `log` crate
  - `defmt`: Implements `defmt::Format` for crate items & enables logging via the `defmt` crate (version 1)
  - `log-level-*`: Enables logs at the selected level and more severe levels (error, warn, info, debug, trace). The trace log level also features detailed MQTT state machine logs.
  - `log-verbose`: Enables high-overhead I/O traces at the trace log level and enables `log-level-trace`

## Usage

It is recommended to use a buffering `Write` implementation, as the current IO model makes fragmented `Write::write` calls. The client also calls `Read::read` frequently; if your underlying implementation involves expensive syscalls, consider using a buffering reader as well.

### Quickstart

The cargo examples in this repository require a broker with correct configuration. Refer to [the contributing guide](CONTRIBUTING.md) for further explanation and guidance.

What follows is an illustrative API example showing explicit session recovery and Quality of Service 2 retransmission after a network failure. The precise network and executor setup is omitted for brevity.

```rust,ignore
async fn main() {
    let mut buffer = AllocBuffer;
    let mut client = Client::new(&mut buffer);

    let transport = ...;    // Any Read/Write implementation (TCP, TLS, ...)

    let connect_options = ConnectOptions::new()
        .clean_start()
        .session_expiry_interval(SessionExpiryInterval::NeverEnd)
        .user_name(MqttString::from_str("user").unwrap())
        .password(MqttBinary::from_slice(b"pass").unwrap());

    client.connect(
        transport,
        &connect_options,
        Some(MqttString::from_str("rust-mqtt-demo").unwrap()),
    ).await.unwrap();

    let topic = MqttString::from_str("demo/topic").unwrap();

    client.subscribe(
        TopicFilter::new(topic.as_borrowed()).unwrap(),
        &SubscriptionOptions::new().exactly_once(),
    ).await.unwrap();

    let topic_reference = TopicReference::Name(TopicName::new(topic).unwrap());

    let packet_identifier = client.publish(
        &PublicationOptions::new(topic_reference.as_borrowed()).exactly_once(),
        "Hello World!".into(),
    ).await.unwrap().unwrap();

    while let Ok(event) = client.poll().await {
        if let Event::PublishComplete(_) = event {
            // Publish succeeded, we can disconnect
            client.disconnect(&DisconnectOptions::new()).await.unwrap();
            return;
        }
    }

    // An error has occured (e.g. network failure)
    client.abort().await;

    let transport = ...;    // Open a fresh connection

    client.connect(
        transport,
        &connect_options,
        Some(MqttString::from_str("rust-mqtt-demo").unwrap()),
    ).await.unwrap();


    // Recover the in-flight Quality of Service 2 publish.

    // Republish if PUBLISH / PUBREC may have been lost
    match client
        .republish(
            packet_identifier,
            &PublicationOptions::new(topic_reference).exactly_once(),
            "Hello World!".into(),
        )
        .await
    {
        // All done / Flight state is already completed
        Ok(_) | Err(MqttError::PacketIdentifierNotInFlight) => {}

        // Re-release if PUBREL / PUBCOMP may have been lost
        Err(MqttError::HandshakeStateMismatched) => client.rerelease().await.unwrap(),

        Err(_) => panic!("Other error :("),
    }
}
```

## Acknowledgment

This project could not be in state in which currently is without Ulf Lilleengen and the rest of the community
from [Drogue IoT](https://github.com/drogue-iot).

## Contact

For any information, open an issue if your matter could be helpful or interesting for others or should be documented. Otherwise contact us on email <julian.jg.graf@gmail.com>, <ond.babec@gmail.com>.

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version 2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in rust-mqtt by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
</sub>
