use std::time::Duration;

use heapless::Vec;

use tokio::time::sleep;
use tokio::{join, task};

use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::client::client_v5::MqttClientV5;
use rust_mqtt::network::network_trait::{Network, NetworkError};
use rust_mqtt::packet::v5::connect_packet::ConnectPacket;
use rust_mqtt::packet::v5::mqtt_packet::Packet;
use rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1;
use rust_mqtt::packet::v5::publish_packet::{PublishPacket, QualityOfService};
use rust_mqtt::packet::v5::subscription_packet::SubscriptionPacket;
use rust_mqtt::tokio_network::TokioNetwork;

async fn receive() {
    let mut ip: [u8; 4] = [127, 0, 0, 1];
    let mut port: u16 = 1883;
    let mut tokio_network: TokioNetwork = TokioNetwork::new(ip, port);
    tokio_network.create_connection().await;
    let mut config = ClientConfig::new();
    config.add_qos(QualityOfService::QoS1);
    config.add_username("test");
    config.add_password("testPass");
    let mut res2 = vec![0; 260];
    let mut res3 = vec![0; 260];
    let mut client = MqttClientV5::<TokioNetwork, 5>::new(
        &mut tokio_network,
        &mut res2,
        260,
        &mut res3,
        260,
        config,
    );

    let mut result = { client.connect_to_broker().await };
    if let Err(r) = result {
        log::error!("[ERROR]: {}", r);
        return;
    }

    {
        const TOPICS: usize = 2;
        let t1 = "test/topic1";
        let t2 = "test/topic2";
        let mut names = Vec::<&str, TOPICS>::new();
        names.push(&t1);
        names.push(&t2);
        client.subscribe_to_topics::<TOPICS>(&names).await;
        //client.subscribe_to_topic("test/topic").await;
    };

    {
        sleep(Duration::from_secs(10));
        client.send_ping().await;
    }
    let mut o = 0;
    loop {
        if o == 2 {
            break;
        }
        log::info!("Waiting for new message!");
        let mes = client.receive_message().await.unwrap();
        let x = String::from_utf8_lossy(mes);
        log::info!("Got new message: {}", x);
        o = o + 1;
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
    let config = ClientConfig::new();
    let mut res2 = vec![0; 260];
    let mut res3 = vec![0; 260];
    let mut client = MqttClientV5::<TokioNetwork, 5>::new(
        &mut tokio_network,
        &mut res2,
        260,
        &mut res3,
        260,
        config,
    );

    let mut result = { client.connect_to_broker().await };
    log::info!("Waiting until send!");
    sleep(Duration::from_secs(15));
    result = {
        log::info!("Sending new message!");
        /*client
        .send_message("test/topic", message)
        .await*/
        client.send_ping().await
    };
    if let Err(e) = result {
        log::error!("Chyba!");
    }

    /*result = {
        log::info!("Sending new message!");
        client
            .send_message("test/topic", "Dalsi zprava :)")
            .await
    };
    if let Err(err) = result {
        log::error!("Chyba!");
    }*/

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

    /*let recv = task::spawn(async move {
        receive().await;
    });

    let publ = task::spawn(async move {
        publish("hello world 123 !").await;
    });

    join!(recv, publ);*/
    receive().await;

    //publish("Ahoj 123").await;
    log::info!("Done");
}
