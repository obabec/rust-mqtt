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
use std::sync::Once;

use futures::future::{join, join3};
use heapless::Vec;
use log::info;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::time::sleep;
use tokio::{net::TcpStream, task};
use tokio_test::{assert_err, assert_ok};

use embedded_io_adapters::tokio_1::FromTokio;
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::client::client_config::MqttVersion::MQTTv5;
use rust_mqtt::packet::v5::property::Property;
use rust_mqtt::packet::v5::publish_packet::QualityOfService;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;
use rust_mqtt::packet::v5::reason_codes::ReasonCode::NotAuthorized;
use rust_mqtt::utils::rng_generator::CountingRng;
pub type TokioNetwork = FromTokio<TcpStream>;

static IP: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
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
    client: &mut MqttClient<'b, TokioNetwork, 5, CountingRng>,
    wait: u64,
    qos: QualityOfService,
    topic: &str,
    message: &str,
    should_err: bool,
) -> Result<(), ReasonCode> {
    info!(
        "[Publisher] Connection to broker with username {} and password {}",
        USERNAME, PASSWORD
    );
    let mut result = { client.connect_to_broker().await };
    assert_ok!(result);
    info!("[Publisher] Waiting {} seconds before sending", wait);
    sleep(Duration::from_secs(wait)).await;

    info!(
        "[Publisher] Sending new message {} to topic {}",
        message, topic
    );
    result = client
        .send_message(topic, message.as_bytes(), qos, false)
        .await;
    info!("[PUBLISHER] sent");
    if should_err {
        assert_err!(result);
    } else {
        assert_ok!(result);
    }

    info!("[Publisher] Disconnecting!");
    result = client.disconnect().await;

    assert_ok!(result);
    Ok(())
}

async fn publish(
    ip: Ipv4Addr,
    wait: u64,
    qos: QualityOfService,
    topic: &str,
) -> Result<(), ReasonCode> {
    let addr = SocketAddr::new(ip.into(), PORT);
    let connection = TcpStream::connect(addr)
        .await
        .map_err(|_| ReasonCode::NetworkError)?;
    let connection = TokioNetwork::new(connection);
    let mut config = ClientConfig::new(MQTTv5, CountingRng(20000));
    config.add_max_subscribe_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 100;
    let mut recv_buffer = [0; 80];
    let mut write_buffer = [0; 80];

    let mut client = MqttClient::<TokioNetwork, 5, CountingRng>::new(
        connection,
        &mut write_buffer,
        80,
        &mut recv_buffer,
        80,
        config,
    );
    publish_core(&mut client, wait, qos, topic, MSG, false).await
}

async fn publish_spec(
    ip: Ipv4Addr,
    wait: u64,
    qos: QualityOfService,
    topic: &str,
    message: &str,
    err: bool,
) -> Result<(), ReasonCode> {
    let addr = SocketAddr::new(ip.into(), PORT);
    let connection = TcpStream::connect(addr)
        .await
        .map_err(|_| ReasonCode::NetworkError)?;
    let connection = TokioNetwork::new(connection);
    let mut config = ClientConfig::new(MQTTv5, CountingRng(20000));
    config.add_max_subscribe_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 100;
    let mut recv_buffer = [0; 80];
    let mut write_buffer = [0; 80];

    let mut client = MqttClient::<TokioNetwork, 5, CountingRng>::new(
        connection,
        &mut write_buffer,
        80,
        &mut recv_buffer,
        80,
        config,
    );
    publish_core(&mut client, wait, qos, topic, message, err).await
}

async fn receive_core<'b>(
    client: &mut MqttClient<'b, TokioNetwork, 5, CountingRng>,
    topic: &str,
) -> Result<(), ReasonCode> {
    info!(
        "[Receiver] Connection to broker with username {} and password {}",
        USERNAME, PASSWORD
    );
    let mut result = client.connect_to_broker().await;
    assert_ok!(result);

    info!("[Receiver] Subscribing to topic {}", topic);
    result = client.subscribe_to_topic(topic).await;
    assert_ok!(result);
    info!("[Receiver] Waiting for new message!");
    let msg = client.receive_message().await;
    assert_ok!(msg);
    let act_message = String::from_utf8_lossy(msg?.1);
    info!("[Receiver] Got new message: {}", act_message);
    assert_eq!(act_message, MSG);

    info!("[Receiver] Disconnecting");
    result = client.disconnect().await;

    assert_ok!(result);
    Ok(())
}

async fn receive_core_multiple<'b, const TOPICS: usize>(
    client: &mut MqttClient<'b, TokioNetwork, 5, CountingRng>,
    topic_names: &'b Vec<&'b str, TOPICS>,
) -> Result<(), ReasonCode> {
    info!(
        "[Receiver] Connection to broker with username {} and password {}",
        USERNAME, PASSWORD
    );
    let mut result = client.connect_to_broker().await;
    assert_ok!(result);

    info!(
        "[Receiver] Subscribing to topics {}, {}",
        topic_names.get(0).unwrap(),
        topic_names.get(1).unwrap()
    );
    result = client.subscribe_to_topics(topic_names).await;

    assert_ok!(result);
    info!("[Receiver] Waiting for new message!");
    {
        let msg = client.receive_message().await;
        assert_ok!(msg);
        let act_message = String::from_utf8_lossy(msg?.1);
        info!("[Receiver] Got new message: {}", act_message);
        assert_eq!(act_message, MSG);
    }
    {
        let msg_sec = client.receive_message().await;
        assert_ok!(msg_sec);
        let act_message_second = String::from_utf8_lossy(msg_sec?.1);
        info!("[Receiver] Got new message: {}", act_message_second);
        assert_eq!(act_message_second, MSG);
    }
    info!("[Receiver] Disconnecting");
    result = client.disconnect().await;

    assert_ok!(result);
    Ok(())
}

async fn receive_multiple<const TOPICS: usize>(
    qos: QualityOfService,
    topic_names: &Vec<&str, TOPICS>,
) -> Result<(), ReasonCode> {
    let addr = SocketAddr::new(IP.into(), PORT);
    let connection = TcpStream::connect(addr)
        .await
        .map_err(|_| ReasonCode::NetworkError)?;
    let connection = TokioNetwork::new(connection);
    let mut config = ClientConfig::new(MQTTv5, CountingRng(20000));
    config.add_max_subscribe_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 60;
    assert_ok!(config.properties.push(Property::ReceiveMaximum(20)));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClient::<TokioNetwork, 5, CountingRng>::new(
        connection,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );

    receive_core_multiple(&mut client, topic_names).await
}

async fn receive(ip: Ipv4Addr, qos: QualityOfService, topic: &str) -> Result<(), ReasonCode> {
    let addr = SocketAddr::new(ip.into(), PORT);
    let connection = TcpStream::connect(addr)
        .await
        .map_err(|_| ReasonCode::NetworkError)?;
    let connection = TokioNetwork::new(connection);
    let mut config = ClientConfig::new(MQTTv5, CountingRng(20000));
    config.add_max_subscribe_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 6000;
    assert_ok!(config.properties.push(Property::ReceiveMaximum(20)));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClient::<TokioNetwork, 5, CountingRng>::new(
        connection,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );

    receive_core(&mut client, topic).await
}

async fn receive_with_wrong_cred(qos: QualityOfService) -> Result<(), ReasonCode> {
    let addr = SocketAddr::new(IP.into(), PORT);
    let connection = TcpStream::connect(addr)
        .await
        .map_err(|_| ReasonCode::NetworkError)?;
    let connection = TokioNetwork::new(connection);
    let mut config = ClientConfig::new(MQTTv5, CountingRng(20000));
    config.add_max_subscribe_qos(qos);
    config.add_username("xyz");
    config.add_password(PASSWORD);
    config.max_packet_size = 60;
    assert_ok!(config.properties.push(Property::ReceiveMaximum(20)));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClient::<TokioNetwork, 5, CountingRng>::new(
        connection,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );

    info!(
        "[Receiver] Connection to broker with username {} and password {}",
        "xyz", PASSWORD
    );
    let result = client.connect_to_broker().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), NotAuthorized);
    Ok(())
}

async fn receive_multiple_second_unsub<const TOPICS: usize>(
    qos: QualityOfService,
    topic_names: &Vec<&str, TOPICS>,
    msg_t1: &str,
    msg_t2: &str,
) -> Result<(), ReasonCode> {
    let addr = SocketAddr::new(IP.into(), PORT);
    let connection = TcpStream::connect(addr)
        .await
        .map_err(|_| ReasonCode::NetworkError)?;
    let connection = TokioNetwork::new(connection);
    let mut config = ClientConfig::new(MQTTv5, CountingRng(20000));
    config.add_max_subscribe_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 60;
    assert_ok!(config.properties.push(Property::ReceiveMaximum(20)));
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];

    let mut client = MqttClient::<TokioNetwork, 5, CountingRng>::new(
        connection,
        &mut write_buffer,
        100,
        &mut recv_buffer,
        100,
        config,
    );

    info!(
        "[Receiver] Connection to broker with username {} and password {}",
        USERNAME, PASSWORD
    );
    let mut result = { client.connect_to_broker().await };
    assert_ok!(result);

    info!(
        "[Receiver] Subscribing to topics {}, {}",
        topic_names.get(0).unwrap(),
        topic_names.get(1).unwrap()
    );
    result = client.subscribe_to_topics(topic_names).await;

    assert_ok!(result);
    info!("[Receiver] Waiting for new message!");
    {
        let msg = { client.receive_message().await };
        assert_ok!(msg);
        let act_message = String::from_utf8_lossy(msg?.1);
        info!("[Receiver] Got new message: {}", act_message);
        assert_eq!(act_message, msg_t1);
    }
    {
        let msg_sec = { client.receive_message().await };
        assert_ok!(msg_sec);
        let act_message_second = String::from_utf8_lossy(msg_sec?.1);
        info!("[Receiver] Got new message: {}", act_message_second);
        assert_eq!(act_message_second, msg_t2);
    }

    {
        let res = client
            .unsubscribe_from_topic(topic_names.get(1).unwrap())
            .await;
        assert_ok!(res);
    }
    {
        let msg = { client.receive_message().await };
        assert_ok!(msg);
        let act_message = String::from_utf8_lossy(msg?.1);
        info!("[Receiver] Got new message: {}", act_message);
        assert_eq!(act_message, msg_t1);
    }

    let res =
        tokio::time::timeout(std::time::Duration::from_secs(10), client.receive_message()).await;
    assert_err!(res);

    info!("[Receiver] Disconnecting");
    result = client.disconnect().await;

    assert_ok!(result);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn integration_publish_recv() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(async move { receive(IP, QualityOfService::QoS0, "test/recv/simple").await });

    let publ =
        task::spawn(
            async move { publish(IP, 5, QualityOfService::QoS0, "test/recv/simple").await },
        );

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn integration_publish_recv_multiple() {
    setup();
    info!("Running simple tests test");
    let mut topic_names = Vec::<&str, 2>::new();
    assert_ok!(topic_names.push("test/topic1"));
    assert_ok!(topic_names.push("test/topic2"));
    let recv =
        task::spawn(async move { receive_multiple(QualityOfService::QoS0, &topic_names).await });

    let publ =
        task::spawn(async move { publish(IP, 5, QualityOfService::QoS0, "test/topic1").await });

    let publ2 =
        task::spawn(async move { publish(IP, 10, QualityOfService::QoS0, "test/topic2").await });

    let (r, p, p2) = join3(recv, publ, publ2).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
    assert_ok!(p2.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn integration_publish_recv_multiple_qos() {
    setup();
    info!("Running simple tests test");
    let mut topic_names = Vec::<&str, 2>::new();
    assert_ok!(topic_names.push("test/topic3"));
    assert_ok!(topic_names.push("test/topic4"));
    let recv =
        task::spawn(async move { receive_multiple(QualityOfService::QoS1, &topic_names).await });

    let publ =
        task::spawn(async move { publish(IP, 5, QualityOfService::QoS1, "test/topic3").await });

    let publ2 =
        task::spawn(async move { publish(IP, 10, QualityOfService::QoS1, "test/topic4").await });

    let (r, p, p2) = join3(recv, publ, publ2).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
    assert_ok!(p2.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn integration_publish_recv_qos() {
    setup();
    info!("Running tests test with Quality of Service 1");

    let recv =
        task::spawn(async move { receive(IP, QualityOfService::QoS1, "test/recv/qos").await });

    let publ =
        task::spawn(async move { publish(IP, 5, QualityOfService::QoS1, "test/recv/qos").await });
    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn integration_publish_recv_wrong_cred() {
    setup();
    info!("Running tests test wrong credentials");

    let recv = task::spawn(async move { receive_with_wrong_cred(QualityOfService::QoS1).await });

    let recv_right =
        task::spawn(async move { receive(IP, QualityOfService::QoS0, "test/recv/wrong").await });

    let publ =
        task::spawn(async move { publish(IP, 5, QualityOfService::QoS1, "test/recv/wrong").await });
    let (r, rv, p) = join3(recv, recv_right, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(rv.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn integration_sub_unsub() {
    setup();
    info!("Running tests with sub and unsub");
    let mut topic_names = Vec::<&str, 2>::new();
    assert_ok!(topic_names.push("unsub/topic1"));
    assert_ok!(topic_names.push("unsub/topic2"));
    let msg_t1 = "First topic message";
    let msg_t2 = "Second topic message";

    let recv = task::spawn(async move {
        receive_multiple_second_unsub(QualityOfService::QoS1, &topic_names, msg_t1, msg_t2).await
    });

    let publ = task::spawn(async move {
        assert_ok!(
            publish_spec(IP, 5, QualityOfService::QoS1, "unsub/topic1", msg_t1, false).await
        );

        publish_spec(IP, 2, QualityOfService::QoS1, "unsub/topic1", msg_t1, false).await
    });

    let publ2 = task::spawn(async move {
        assert_ok!(
            publish_spec(IP, 6, QualityOfService::QoS1, "unsub/topic2", msg_t2, false).await
        );

        publish_spec(IP, 3, QualityOfService::QoS1, "unsub/topic2", msg_t2, true).await
    });
    let (r, p1, p2) = join3(recv, publ, publ2).await;
    assert_ok!(r.unwrap());
    assert_ok!(p1.unwrap());
    assert_ok!(p2.unwrap());
}
