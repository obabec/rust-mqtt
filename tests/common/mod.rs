use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use embedded_io_adapters::tokio_1::FromTokio;
use rust_mqtt::{
    buffer::AllocBuffer,
    client::{
        Client,
        options::{ConnectOptions, DisconnectOptions, RetainHandling, SubscriptionOptions},
    },
    config::{KeepAlive, MaximumPacketSize, SessionExpiryInterval},
    types::{MqttBinary, MqttString, QoS},
};
use tokio::net::TcpStream;

use crate::common::failing::FailingTcp;

pub mod assert;
pub mod failing;
pub mod fmt;
pub mod utils;

type DefaultClient<'a, T> = Client<'a, T, AllocBuffer, 1, 1, 1, 1>;

pub type TestClient<'a> = DefaultClient<'a, FromTokio<TcpStream>>;
pub type FailingClient<'a> = DefaultClient<'a, FromTokio<FailingTcp>>;

pub const BROKER_ADDRESS: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 1883));

pub const USERNAME: MqttString<'static> = MqttString::from_str_unchecked("test");
pub const PASSWORD: MqttBinary<'static> = MqttBinary::from_slice_unchecked("testPass".as_bytes());

pub const NO_SESSION_CONNECT_OPTIONS: &ConnectOptions<'static> = &ConnectOptions {
    clean_start: true,
    keep_alive: KeepAlive::Infinite,
    maximum_packet_size: MaximumPacketSize::Unlimited,
    session_expiry_interval: SessionExpiryInterval::EndOnDisconnect,
    request_response_information: false,
    user_name: Some(USERNAME),
    password: Some(PASSWORD),
    will: None,
};

pub const DEFAULT_QOS0_SUB_OPTIONS: SubscriptionOptions = SubscriptionOptions {
    retain_handling: RetainHandling::AlwaysSend,
    retain_as_published: false,
    no_local: false,
    qos: QoS::AtMostOnce,
    subscription_identifier: None,
};

pub const DEFAULT_DC_OPTIONS: &DisconnectOptions = &DisconnectOptions::new().publish_will();
