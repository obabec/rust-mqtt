use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use embedded_io_adapters::tokio_1::FromTokio;
use rust_mqtt::{
    buffer::AllocBuffer,
    client::{
        Client,
        options::{ConnectOptions, DisconnectOptions, RetainHandling, SubscriptionOptions},
    },
    config::{KeepAlive, SessionExpiryInterval},
    types::{MqttBinary, MqttString, QoS},
};
use tokio::net::TcpStream;

pub mod assert;
pub mod fmt;
pub mod utils;

pub type Tcp = FromTokio<TcpStream>;
pub type TestClient<'a> = Client<'a, Tcp, AllocBuffer, 1, 1, 1, 0>;

pub const BROKER_ADDRESS: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 1883));

pub const USERNAME: MqttString<'static> = unsafe { MqttString::from_slice_unchecked("test") };
pub const PASSWORD: MqttBinary<'static> =
    unsafe { MqttBinary::from_slice_unchecked("testPass".as_bytes()) };

pub const NO_SESSION_CONNECT_OPTIONS: &ConnectOptions<'static> = &ConnectOptions {
    clean_start: true,
    keep_alive: KeepAlive::Infinite,
    session_expiry_interval: SessionExpiryInterval::EndOnDisconnect,
    user_name: Some(USERNAME),
    password: Some(PASSWORD),
    will: None,
};

pub const DEFAULT_QOS0_SUB_OPTIONS: SubscriptionOptions = SubscriptionOptions {
    retain_handling: RetainHandling::AlwaysSend,
    retain_as_published: false,
    no_local: false,
    qos: QoS::AtMostOnce,
};

pub const DEFAULT_DC_OPTIONS: &DisconnectOptions = &DisconnectOptions {
    publish_will: true,
    session_expiry_interval: None,
};
