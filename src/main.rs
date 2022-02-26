use rust_mqtt::client::client_v5::MqttClientV5;
use rust_mqtt::network::network_trait::{Network, NetworkError};
use rust_mqtt::packet::connect_packet::ConnectPacket;
use rust_mqtt::packet::mqtt_packet::Packet;
use rust_mqtt::packet::publish_packet::{PublishPacket, QualityOfService};
use rust_mqtt::packet::subscription_packet::SubscriptionPacket;
use rust_mqtt::tokio_network::TokioNetwork;
use std::time::Duration;
use tokio::time::sleep;
use tokio::{join, task};

async fn receive() {
    let mut ip: [u8; 4] = [37, 205, 11, 180];
    let mut port: u16 = 1883;
    let mut tokio_network: TokioNetwork = TokioNetwork::new(ip, port);
    tokio_network.create_connection().await;
    let mut res2 = vec![0; 260];
    let mut client = MqttClientV5::<TokioNetwork, 5>::new(&mut tokio_network, &mut res2);

    let mut result = { client.connect_to_broker().await };

    {
        client.subscribe_to_topic("test/topic").await;
    }
    {
        log::info!("Waiting for new message!");
        let mes = client.receive_message().await.unwrap();
        let x = String::from_utf8_lossy(mes);
        log::info!("Got new message: {}", x);
    }
    {
        client.disconnect().await;
    }
}

async fn publish(message: &str) {
    let mut ip: [u8; 4] = [37, 205, 11, 180];
    let mut port: u16 = 1883;
    let mut tokio_network: TokioNetwork = TokioNetwork::new(ip, port);
    tokio_network.create_connection().await;
    let mut res2 = vec![0; 260];
    let mut client = MqttClientV5::<TokioNetwork, 5>::new(&mut tokio_network, &mut res2);

    let mut result = { client.connect_to_broker().await };
    log::info!("Waiting until send!");
    sleep(Duration::from_secs(15));
    let mut result: Result<(), NetworkError> = {
        log::info!("Sending new message!");
        client
            .send_message("test/topic", message, QualityOfService::QoS0)
            .await
    };

    {
        client.disconnect().await;
    }
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_nanos()
        .init();

    let recv = task::spawn(async move {
        receive().await;
    });

    let publ = task::spawn(async move {
        publish("hello world 123 !").await;
    });

    join!(recv, publ);
    log::info!("Done");
}
