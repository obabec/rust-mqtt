# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

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
