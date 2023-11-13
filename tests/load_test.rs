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

use futures::future::join;
use log::info;
use serial_test::serial;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::task;
use tokio::time::sleep;
use tokio_test::assert_ok;

use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::client::client_config::MqttVersion::MQTTv5;
use rust_mqtt::packet::v5::publish_packet::QualityOfService;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;
use rust_mqtt::utils::rng_generator::CountingRng;
use tokio::net::TcpStream;

use embedded_io_adapters::tokio_1::FromTokio;
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
    topic: &str,
    amount: u16,
) -> Result<(), ReasonCode> {
    info!(
        "[Publisher] Connection to broker with username {} and password {}",
        USERNAME, PASSWORD
    );
    let mut result = client.connect_to_broker().await;
    assert_ok!(result);
    info!("[Publisher] Waiting {} seconds before sending", wait);
    sleep(Duration::from_secs(wait)).await;

    info!("[Publisher] Sending new message {} to topic {}", MSG, topic);
    let mut count = 0;
    loop {
        result = client
            .send_message(topic, MSG.as_bytes(), QualityOfService::QoS0, false)
            .await;
        info!("[PUBLISHER] sent {}", count);
        assert_ok!(result);
        count += 1;
        if count == amount {
            break;
        }
        //sleep(Duration::from_millis(5)).await;
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
    amount: u16,
) -> Result<(), ReasonCode> {
    let addr = SocketAddr::new(ip.into(), PORT);
    let connection = TcpStream::connect(addr)
        .await
        .map_err(|_| ReasonCode::NetworkError)?;
    let connection = TokioNetwork::new(connection);
    let mut config = ClientConfig::new(MQTTv5, CountingRng(50000));
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
    publish_core(&mut client, wait, topic, amount).await
}

async fn receive_core<'b>(
    client: &mut MqttClient<'b, TokioNetwork, 5, CountingRng>,
    topic: &str,
    amount: u16,
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
    let mut count = 0;
    loop {
        let msg = client.receive_message().await;
        assert_ok!(msg);
        let act_message = String::from_utf8_lossy(msg?.1);
        info!("[Receiver] Got new {}. message: {}", count, act_message);
        assert_eq!(act_message, MSG);
        count += 1;
        if count == amount {
            break;
        }
    }
    info!("[Receiver] Disconnecting");
    result = client.disconnect().await;
    assert_ok!(result);
    Ok(())
}

async fn receive(
    ip: Ipv4Addr,
    qos: QualityOfService,
    topic: &str,
    amount: u16,
) -> Result<(), ReasonCode> {
    let addr = SocketAddr::new(ip.into(), PORT);
    let connection = TcpStream::connect(addr)
        .await
        .map_err(|_| ReasonCode::NetworkError)?;
    let connection = TokioNetwork::new(connection);
    let mut config = ClientConfig::new(MQTTv5, CountingRng(50000));
    config.add_max_subscribe_qos(qos);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 6000;
    config.keep_alive = 60000;
    config.max_packet_size = 300;
    let mut recv_buffer = [0; 500];
    let mut write_buffer = [0; 500];

    let mut client = MqttClient::<TokioNetwork, 5, CountingRng>::new(
        connection,
        &mut write_buffer,
        500,
        &mut recv_buffer,
        500,
        config,
    );

    receive_core(&mut client, topic, amount).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_ten() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(async move { receive(IP, QualityOfService::QoS0, "test/recv/ten", 10).await });

    let publ =
        task::spawn(
            async move { publish(IP, 5, QualityOfService::QoS0, "test/recv/ten", 10).await },
        );

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn load_test_ten_qos() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(
            async move { receive(IP, QualityOfService::QoS1, "test/recv/ten/qos", 10).await },
        );

    let publ = task::spawn(async move {
        publish(IP, 5, QualityOfService::QoS1, "test/recv/ten/qos", 10).await
    });

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_fifty() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(
            async move { receive(IP, QualityOfService::QoS0, "test/recv/fifty", 50).await },
        );

    let publ =
        task::spawn(
            async move { publish(IP, 5, QualityOfService::QoS0, "test/recv/fifty", 50).await },
        );

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_fifty_qos() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(
            async move { receive(IP, QualityOfService::QoS1, "test/recv/fifty/qos", 50).await },
        );

    let publ = task::spawn(async move {
        publish(IP, 5, QualityOfService::QoS1, "test/recv/fifty/qos", 50).await
    });

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_hundred() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(
            async move { receive(IP, QualityOfService::QoS0, "test/recv/hundred", 100).await },
        );

    let publ = task::spawn(async move {
        publish(IP, 5, QualityOfService::QoS0, "test/recv/hundred", 100).await
    });

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_hundred_qos() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(async move { receive(IP, QualityOfService::QoS1, "hundred/qos", 100).await });

    let publ =
        task::spawn(
            async move { publish(IP, 5, QualityOfService::QoS1, "hundred/qos", 100).await },
        );

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_five_hundred() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(async move { receive(IP, QualityOfService::QoS0, "five/hundred", 500).await });

    let publ =
        task::spawn(
            async move { publish(IP, 5, QualityOfService::QoS0, "five/hundred", 500).await },
        );

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_five_hundred_qos() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(
            async move { receive(IP, QualityOfService::QoS1, "five/hundred/qos", 500).await },
        );

    let publ = task::spawn(async move {
        publish(IP, 5, QualityOfService::QoS1, "five/hundred/qos", 500).await
    });

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_thousand() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(async move { receive(IP, QualityOfService::QoS0, "thousand", 1000).await });

    let publ =
        task::spawn(async move { publish(IP, 5, QualityOfService::QoS0, "thousand", 1000).await });

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_thousand_qos() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(async move { receive(IP, QualityOfService::QoS1, "thousand/qos", 1000).await });

    let publ =
        task::spawn(
            async move { publish(IP, 5, QualityOfService::QoS1, "thousand/qos", 1000).await },
        );

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

// 72s
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_ten_thousand_qos() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(
            async move { receive(IP, QualityOfService::QoS1, "ten/thousand/qos", 10000).await },
        );

    let publ = task::spawn(async move {
        publish(IP, 5, QualityOfService::QoS1, "ten/thousand/qos", 10000).await
    });

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_ten_thousand() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(
            async move { receive(IP, QualityOfService::QoS0, "ten/thousand", 10000).await },
        );

    let publ =
        task::spawn(
            async move { publish(IP, 5, QualityOfService::QoS0, "ten/thousand", 10000).await },
        );

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

// 72s
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_twenty_thousand_qos() {
    setup();
    info!("Running simple tests test");

    let recv = task::spawn(async move {
        receive(IP, QualityOfService::QoS1, "twenty/thousand/qos", 20000).await
    });

    let publ = task::spawn(async move {
        publish(IP, 5, QualityOfService::QoS1, "twenty/thousand/qos", 20000).await
    });

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial]
async fn load_test_twenty_thousand() {
    setup();
    info!("Running simple tests test");

    let recv =
        task::spawn(
            async move { receive(IP, QualityOfService::QoS0, "twenty/thousand", 20000).await },
        );

    let publ = task::spawn(async move {
        publish(IP, 5, QualityOfService::QoS0, "twenty/thousand", 20000).await
    });

    let (r, p) = join(recv, publ).await;
    assert_ok!(r.unwrap());
    assert_ok!(p.unwrap());
}
