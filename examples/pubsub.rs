use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use embedded_io_adapters::tokio_1::FromTokio;
use rust_mqtt::{
    client::{client::MqttClient, client_config::ClientConfig},
    packet::v5::reason_codes::ReasonCode,
    utils::rng_generator::CountingRng,
};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 1883);

    let connection = TcpStream::connect(addr)
        .await
        .map_err(|_| ReasonCode::NetworkError)
        .unwrap();
    let connection = FromTokio::<TcpStream>::new(connection);
    let mut config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );
    config.add_max_subscribe_qos(rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1);
    config.add_client_id("client");
    // config.add_username(USERNAME);
    // config.add_password(PASSWORD);
    config.max_packet_size = 100;
    let mut recv_buffer = [0; 80];
    let mut write_buffer = [0; 80];

    let mut client = MqttClient::<_, 5, _>::new(
        connection,
        &mut write_buffer,
        80,
        &mut recv_buffer,
        80,
        config,
    );

    client.connect_to_broker().await.unwrap();

    loop {
        client
            .send_message(
                "hello",
                b"hello2",
                rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0,
                true,
            )
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
