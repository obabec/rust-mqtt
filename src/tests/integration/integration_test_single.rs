/*
 * MIT License
 *
 * Copyright (c) [2022] [Ondrej Babec <ond.babec@gmail.com>]
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use alloc::string::String;
use core::time::Duration;
use heapless::Vec;
use tokio::{join, task};
use tokio::time::sleep;
use crate::client::client_config::ClientConfig;
use crate::client::client_v5::MqttClientV5;
use crate::network::network_trait::Network;
use crate::packet::v5::property::Property;
use crate::packet::v5::publish_packet::QualityOfService;
use crate::tokio_network::TokioNetwork;

static IP: [u8; 4] = [127, 0, 0, 1];
static PORT: u16 = 1883;
static USERNAME: &str = "test";
static PASSWORD: &str = "testPass";
static TOPIC: &str = "test/topic";
static MESSAGE: &str = "testMessage";

async fn publish() {
    let mut tokio_network: TokioNetwork = TokioNetwork::new(IP, PORT);
    tokio_network.create_connection().await;
    let mut config = ClientConfig::new();
    config.add_qos(QualityOfService::QoS0);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 80;
    let mut recv_buffer = [0; 80];
    let mut write_buffer = [0; 80];

    let mut client = MqttClientV5::<TokioNetwork, 5>::new(
        &mut tokio_network,
        &mut write_buffer,
        80,
        &mut recv_buffer,
        80,
        config,
    );

    log::info!("[Publisher] Connection to broker with username {} and password {}", USERNAME, PASSWORD);
    let mut result = {
        client.connect_to_broker().await
    };
    assert!(result.is_ok());

    log::info!("[Publisher] Waiting {} seconds before sending", 5);
    sleep(Duration::from_secs(5)).await;

    log::info!("[Publisher] Sending new message {}", MESSAGE);
    result = {
        client.send_message(TOPIC, MESSAGE).await
    };
    assert!(result.is_ok());

    log::info!("[Publisher] Disconnecting!");
    result = {
        client.disconnect().await
    };
    assert!(result.is_ok());
}

async fn receive() {
    let mut tokio_network: TokioNetwork = TokioNetwork::new(IP, PORT);
    tokio_network.create_connection().await;
    let mut config = ClientConfig::new();
    config.add_qos(QualityOfService::QoS0);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 60;
    config.properties.push(Property::ReceiveMaximum(20));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClientV5::<TokioNetwork, 2>::new(
        &mut tokio_network,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );
    log::info!("[Receiver] Connection to broker with username {} and password {}", USERNAME, PASSWORD);
    let mut result = {client.connect_to_broker().await};
    assert!(result.is_ok());

    log::info!("[Receiver] Subscribing to topic {}", TOPIC);
    result = {
        client.subscribe_to_topic(TOPIC).await
    };
    assert!(result.is_ok());

    log::info!("[Receiver] Waiting for new message!");
    let msg = {
        client.receive_message().await
    };
    assert!(msg.is_ok());
    let act_message = String::from_utf8_lossy(msg.unwrap());
    log::info!("[Receiver] Got new message: {}", act_message);
    assert_eq!(act_message, MESSAGE);

    log::info!("[Receiver] Disconnecting");
    result = {
        client.disconnect().await
    };
    assert!(result.is_ok());
}

#[tokio::test]
async fn simple_publish_recv() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_nanos()
        .init();

    log::info!("Running simple integration test");

    let recv = task::spawn(async move {
        receive().await;
    });

    let publ = task::spawn(async move {
        publish().await;
    });
    join!(recv, publ);
}