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
use log::LevelFilter;
use tokio::time::sleep;
use tokio::task;
use tokio_test::{assert_err, assert_ok};
use heapless::Vec;
use crate::client::client_config::ClientConfig;
use crate::client::client_v5::MqttClientV5;
use crate::network::network_trait::{NetworkConnection, NetworkConnectionFactory};
use crate::packet::v5::property::Property;
use crate::packet::v5::publish_packet::QualityOfService;
use crate::packet::v5::reason_codes::ReasonCode;
use crate::packet::v5::reason_codes::ReasonCode::NotAuthorized;
use crate::tokio_net::tokio_network::{TokioNetwork, TokioNetworkFactory};
use crate::utils::types::BufferError;
use std::sync::Once;
use futures::future::{join, join3};

static IP: [u8; 4] = [127, 0, 0, 1];
static WRONG_IP: [u8; 4] = [192, 168, 1, 1];
static PORT: u16 = 1883;
static USERNAME: &str = "test";
static PASSWORD: &str = "testPass";
static MSG: &str = "testMessage";

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

async fn publish_core<'b>(
    client: &mut MqttClientV5<'b, TokioNetwork, 5>,
    wait: u64,
    topic: &str,
) -> Result<(), ReasonCode> {
    info!(
        "[Publisher] Connection to broker with username {} and password {}",
        USERNAME,
        PASSWORD
    );
    let mut result = { client.connect_to_broker().await };
    assert_ok!(result);
    info!("[Publisher] Waiting {} seconds before sending", wait);
    sleep(Duration::from_secs(wait)).await;

    info!("[Publisher] Sending new message {} to topic {}", MSG, topic);
    result = { client.send_message(topic, MSG).await };
    assert_ok!(result);

    info!("[Publisher] Disconnecting!");
    result = { client.disconnect().await };
    assert_ok!(result);
    Ok(())
}

async fn publish(ip: [u8; 4], wait: u64, qos: QualityOfService, topic: &str) -> Result<(), ReasonCode> {
    let mut tokio_factory: TokioNetworkFactory = TokioNetworkFactory::new();
    let mut tokio_network: TokioNetwork = tokio_factory.connect(ip, PORT).await?;
    let mut config = ClientConfig::new();
    config.add_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 100;
    let mut recv_buffer = [0; 80];
    let mut write_buffer = [0; 80];

    let mut client = MqttClientV5::<TokioNetwork, 5>::new(
        tokio_network,
        &mut write_buffer,
        80,
        &mut recv_buffer,
        80,
        config,
    );
    publish_core(&mut client, wait, topic).await
}

async fn receive_core<'b>(
    client: &mut MqttClientV5<'b, TokioNetwork, 5>,
    topic: &str,
) -> Result<(), ReasonCode> {
    info!(
        "[Receiver] Connection to broker with username {} and password {}",
        USERNAME,
        PASSWORD
    );
    let mut result = { client.connect_to_broker().await };
    assert_ok!(result);

    info!("[Receiver] Subscribing to topic {}", topic);
    result = { client.subscribe_to_topic(topic).await };
    assert_ok!(result);
    info!("[Receiver] Waiting for new message!");
    let msg = { client.receive_message().await };
    assert_ok!(msg);
    let act_message = String::from_utf8_lossy(msg?);
    info!("[Receiver] Got new message: {}", act_message);
    assert_eq!(act_message, MSG);

    info!("[Receiver] Disconnecting");
    result = { client.disconnect().await };
    assert_ok!(result);
    Ok(())
}



async fn receive_core_multiple<'b, const TOPICS: usize>(
    client: &mut MqttClientV5<'b, TokioNetwork, 5>,
    topic_names: &'b Vec<&'b str, TOPICS>,
) -> Result<(), ReasonCode> {
    info!(
        "[Receiver] Connection to broker with username {} and password {}",
        USERNAME,
        PASSWORD
    );
    let mut result = { client.connect_to_broker().await };
    assert_ok!(result);

    info!("[Receiver] Subscribing to topics {}, {}", topic_names.get(0).unwrap(), topic_names.get(1).unwrap());
    result = { client.subscribe_to_topics(topic_names).await };
    assert_ok!(result);
    info!("[Receiver] Waiting for new message!");
    {
        let msg = { client.receive_message().await };
        assert_ok!(msg);
        let act_message = String::from_utf8_lossy(msg?);
        info!("[Receiver] Got new message: {}", act_message);
        assert_eq!(act_message, MSG);
    }
    {
        let msg_sec = { client.receive_message().await };
        assert_ok!(msg_sec);
        let act_message_second = String::from_utf8_lossy(msg_sec?);
        info!("[Receiver] Got new message: {}", act_message_second);
        assert_eq!(act_message_second, MSG);
    }
    info!("[Receiver] Disconnecting");
    result = { client.disconnect().await };
    assert_ok!(result);
    Ok(())
}

async fn receive_multiple<const TOPICS: usize>(qos: QualityOfService, topic_names: & Vec<& str, TOPICS>,) -> Result<(), ReasonCode> {
    let mut tokio_factory: TokioNetworkFactory = TokioNetworkFactory::new();
    let mut tokio_network: TokioNetwork = tokio_factory.connect(IP, PORT).await?;
    let mut config = ClientConfig::new();
    config.add_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 60;
    config.properties.push(Property::ReceiveMaximum(20));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClientV5::<TokioNetwork, 5>::new(
        tokio_network,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );

    receive_core_multiple(&mut client, topic_names).await
}

async fn receive(ip: [u8; 4], qos: QualityOfService, topic: &str) -> Result<(), ReasonCode> {
    let mut tokio_factory: TokioNetworkFactory = TokioNetworkFactory::new();
    let mut tokio_network: TokioNetwork = tokio_factory.connect(ip, PORT).await?;
    let mut config = ClientConfig::new();
    config.add_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 60;
    config.properties.push(Property::ReceiveMaximum(20));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClientV5::<TokioNetwork, 5>::new(
        tokio_network,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );

    receive_core(&mut client, topic).await
}

async fn receive_with_wrong_cred(qos: QualityOfService) -> Result<(), ReasonCode> {
    let mut tokio_factory: TokioNetworkFactory = TokioNetworkFactory::new();
    let mut tokio_network: TokioNetwork = tokio_factory.connect(IP, PORT).await?;
    let mut config = ClientConfig::new();
    config.add_qos(qos);
    config.add_username("xyz");
    config.add_password(PASSWORD);
    config.max_packet_size = 60;
    config.properties.push(Property::ReceiveMaximum(20));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClientV5::<TokioNetwork, 5>::new(
        tokio_network,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );

    info!(
        "[Receiver] Connection to broker with username {} and password {}",
        "xyz",
        PASSWORD
    );
    let result = { client.connect_to_broker().await };
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), NotAuthorized);
    Err(NotAuthorized)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple_publish_recv() {
    setup();
    info!("Running simple integration test");

    let recv =
        task::spawn(async move { receive(IP, QualityOfService::QoS0, "test/recv/simple").await });

    let publ =
        task::spawn(async move { publish(IP, 5, QualityOfService::QoS0, "test/recv/simple").await });

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple_publish_recv_multiple() {
    setup();
    info!("Running simple integration test");
    let mut topic_names = Vec::<&str, 2>::new();
    topic_names.push("test/topic1");
    topic_names.push("test/topic2");
    let recv =
        task::spawn(async move { receive_multiple(QualityOfService::QoS0, &topic_names).await });

    let publ =
        task::spawn(async move { publish(IP, 5,QualityOfService::QoS0, "test/topic1").await });

    let publ2 =
        task::spawn(async move { publish(IP, 10, QualityOfService::QoS0, "test/topic2").await });

    let (r, p, p2) = join3(recv, publ, publ2).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
    assert_ok!(p2.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple_publish_recv_multiple_qos() {
    setup();
    info!("Running simple integration test");
    let mut topic_names = Vec::<&str, 2>::new();
    topic_names.push("test/topic3");
    topic_names.push("test/topic4");
    let recv =
       task::spawn(async move { receive_multiple(QualityOfService::QoS1, &topic_names).await });

    let publ =
        task::spawn(async move { publish(IP, 5, QualityOfService::QoS1, "test/topic3").await });

    let publ2 =
        task::spawn(async move { publish(IP, 10, QualityOfService::QoS1, "test/topic4").await });

    let ( r, p, p2) = join3(recv, publ, publ2).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
    assert_ok!(p2.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple_publish_recv_qos() {
    setup();
    info!("Running simple integration test with Quality of Service 1");

    let recv = task::spawn(async move { receive(IP, QualityOfService::QoS1, "test/recv/qos").await });

    let publ = task::spawn(async move { publish(IP, 5, QualityOfService::QoS1, "test/recv/qos").await });
    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple_publish_recv_wrong_cred() {
    setup();
    info!("Running simple integration test wrong credentials");

    let recv = task::spawn(async move { receive_with_wrong_cred(QualityOfService::QoS1).await });

    let recv_right =
        task::spawn(async move { receive(IP, QualityOfService::QoS0, "test/recv/wrong").await });

    let publ = task::spawn(async move { publish(IP, 5, QualityOfService::QoS1, "test/recv/wrong").await });
    let (r, rv, p) = join3(recv, recv_right, publ).await;
    assert_err!(r.unwrap());
    assert_ok!(rv.unwrap());
    assert_ok!(p.unwrap());
}
