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
extern crate alloc;
use alloc::string::String;
use core::time::Duration;
use std::future::Future;
use tokio::{join, task};
use tokio::time::sleep;
use tokio_test::assert_ok;


use crate::client::client_config::ClientConfig;
use crate::client::client_v5::MqttClientV5;
use crate::packet::v5::property::Property;
use crate::packet::v5::publish_packet::QualityOfService;
use crate::packet::v5::reason_codes::ReasonCode::NotAuthorized;
use crate::tokio_net::tokio_network::TokioNetwork;
use crate::network::network_trait::Network;
use crate::packet::v5::reason_codes::ReasonCode;
use crate::utils::types::BufferError;


static IP: [u8; 4] = [127, 0, 0, 1];
static PORT: u16 = 1883;
static USERNAME: &str = "test";
static PASSWORD: &str = "testPass";
static TOPIC: &str = "test/topic";
static MSG: &str = "testMessage";

fn init() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_nanos()
        .try_init();
}

async fn publish_core<'b>(client: & mut MqttClientV5<'b, TokioNetwork, 5>) -> Result<(), ReasonCode> {
    log::info!("[Publisher] Connection to broker with username {} and password {}", USERNAME, PASSWORD);
    let mut result = {
        client.connect_to_broker().await
    };
    assert_ok!(result);
    log::info!("[Publisher] Waiting {} seconds before sending", 5);
    sleep(Duration::from_secs(5)).await;

    log::info!("[Publisher] Sending new message {}", MSG);
    result = {
        client.send_message(TOPIC, MSG).await
    };
    assert_ok!(result);

    log::info!("[Publisher] Disconnecting!");
    result = {
        client.disconnect().await
    };
    assert_ok!(result);
    Ok(())
}

async fn publish(qos: QualityOfService) -> Result<(), ReasonCode> {
    let mut tokio_network: TokioNetwork = TokioNetwork::new(IP, PORT);
    tokio_network.create_connection().await ?;
    let mut config = ClientConfig::new();
    config.add_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 100;
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
    publish_core(& mut client)
        .await
}

async fn receive_core<'b>(client: & mut MqttClientV5<'b, TokioNetwork, 5>) -> Result<(), ReasonCode> {
    log::info!("[Receiver] Connection to broker with username {} and password {}", USERNAME, PASSWORD);
    let mut result = {client.connect_to_broker().await};
    assert_ok!(result);

    log::info!("[Receiver] Subscribing to topic {}", TOPIC);
    result = {
        client.subscribe_to_topic(TOPIC).await
    };
    assert_ok!(result);
    log::info!("[Receiver] Waiting for new message!");
    let msg = {
        client.receive_message().await
    };
    assert_ok!(msg);
    let act_message = String::from_utf8_lossy(msg?);
    log::info!("[Receiver] Got new message: {}", act_message);
    assert_eq!(act_message, MSG);

    log::info!("[Receiver] Disconnecting");
    result = {
        client.disconnect().await
    };
    assert_ok!(result);
    Ok(())
}

async fn receive(qos: QualityOfService) -> Result<(), ReasonCode> {
    let mut tokio_network: TokioNetwork = TokioNetwork::new(IP, PORT);
    tokio_network.create_connection().await;
    let mut config = ClientConfig::new();
    config.add_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 60;
    config.properties.push(Property::ReceiveMaximum(20));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClientV5::<TokioNetwork, 5>::new(
        &mut tokio_network,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );

    receive_core(& mut client)
        .await
}


async fn receive_with_wrong_cred(qos: QualityOfService) {
    let mut tokio_network: TokioNetwork = TokioNetwork::new(IP, PORT);
    tokio_network.create_connection().await;
    let mut config = ClientConfig::new();
    config.add_qos(qos);
    config.add_username("xyz");
    config.add_password(PASSWORD);
    config.max_packet_size = 60;
    config.properties.push(Property::ReceiveMaximum(20));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClientV5::<TokioNetwork, 5>::new(
        &mut tokio_network,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );

    log::info!("[Receiver] Connection to broker with username {} and password {}", "xyz", PASSWORD);
    let result = {client.connect_to_broker().await};
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), NotAuthorized);
}

#[tokio::test]
async fn simple_publish_recv() {
    init();
    log::info!("Running simple integration test");

    let recv = task::spawn(async move {
        receive(QualityOfService::QoS0).await
    });

    let publ = task::spawn(async move {
        publish(QualityOfService::QoS0)
            .await
    });

    let (r, p) = join!(recv, publ);
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test]
async fn simple_publish_recv_qos() {
    init();
    log::info!("Running simple integration test with Quality of Service 1");

    let recv = task::spawn(async move {
        receive(QualityOfService::QoS1)
            .await
    });

    let publ = task::spawn(async move {
        publish(QualityOfService::QoS1)
            .await
    });
    let (r, p) = join!(recv, publ);
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test]
async fn simple_publish_recv_wrong_cred() {
    init();
    log::info!("Running simple integration test wrong credentials");

    let recv = task::spawn(async move {
        receive_with_wrong_cred(QualityOfService::QoS1)
            .await
    });

    let recv_right = task::spawn(async move {
        receive(QualityOfService::QoS0)
            .await
    });

    let publ = task::spawn(async move {
        publish(QualityOfService::QoS1)
            .await
    });
    let (r, rv, p) = join!(recv, recv_right, publ);
    assert_ok!(rv.unwrap());
    assert_ok!(p.unwrap());
}