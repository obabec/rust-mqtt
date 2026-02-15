# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

- Add subscription identifiers in subscriptions & incoming publishes
- Add message expiry interval to outgoing and incoming publications
- Add support for topic aliases to outgoing publications
- Treat an incoming PUBLISH packet with an empty topic and no topic alias as a protocol error
- Fix the session expiry interval property with a value of 0 being elided in DISCONNECT packets on the network
- Add clearer errors for invalid republishes
- Fix in flight packet identifiers being lost if transmission and retransmission of PUBLISH packets fail
- Allow incoming QoS 1 publications to cause duplicate application messages
- Add `Event::Duplicate` for duplicate incoming QoS 2 publications instead of using `Event::Ignored`
- Correctly accept QoS 2 retransmissions if receive maximum is reached
- Don't remove in-flight entries on protocol errors caused by received PUBACK, PUBREC & PUBCOMP packets mismatching with the client's session state
- Rename error variant `Error::PacketMaxLengthExceeded` to `Error::PacketMaximumLengthExceeded`
- Send appropriate PUBREL & PUBCOMP packets with reason code Packet Identifier Not Found when receiving such PUBREC & PUBREL packets instead of ignoring to prevent publish flow deadlocks
- Make methods throughout `types` module const where possible
- Add syntax validating factory methods for `TopicName` and `TopicFilter`
- Parse and validate topic names in incoming PUBLISH packets
- Remove unsafe from unchecked factory methods from `TopicName`, `TopicFilter` and `VarByteInt` as they don't cause UB
- Remove `VarByteInt::from_slice_unchecked` from public API
- Add request/response pattern support
- Add builder pattern to `ConnectOptions`, `DisconnectOptions`, `PublicationOptions`, `SubscriptionOptions` and `WillOptions`
- Change the type of `WillOptions`'s `will_topic` and `Publish`'s `topic` from `MqttString` to `TopicName`
- Improve constructor naming & safety in `types` module
- Add null character invariant to `MqttString`
- Add `FixedHeader` and `PacketType` to public API
- Remove `Raw`-related types from public API
- Fix client panics when server sends SUBACK/UNSUBACK packets with no reason codes
- Remove `Error::ReceiveBuffer` as it is superseeded by `Error::Server` in SUBACK/UNSUBACK handling

## 0.4.1 - 2026-01-06

- Comply with server's maximum packet size
- Add tls example

## 0.4.0 - 2025-12-16

- Extensive rewrite
- Trait based buffer provision in deserialization
- More robust packet deserialization regarding protocol errors and malformed packets
- Newtype abstraction of types in the mqtt protocol
- Improved error handling with non-recoverable and recoverable errors
- Quality of service 2
- More lightweight mqtt features such as flags in packets
- Remove support for user properties
- Upgrade to Rust 1.92

## 0.3.1 - 2025-12-11

- Add support for the embedded_io_async::ReadReady trait
- Don't treat a reason code of NoMatchingSubscribers from server as error
- Bugfixes in networking, IO and deserializing of disconnect packet
- Bump dependencies and Rust version to 1.91

## 0.3.0 - 2024-01-30

- Bump dependencies
- Switch to stable Rust

## 0.2.0 - 2023-12-03

- Bump dependencies and fix warnings
