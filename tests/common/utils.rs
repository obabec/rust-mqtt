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

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    str::from_utf8,
    sync::Once,
    time::Duration,
};

use embedded_io_adapters::tokio_1::FromTokio;
use log::{error, info};
use rust_mqtt::{
    client::MqttClient,
    interface::{ClientConfig, MqttVersion, QualityOfService, ReasonCode, RetainHandling, Topic},
};
use tokio::{
    net::TcpStream,
    time::{sleep, timeout},
};
use tokio_test::{assert_err, assert_ok};

use crate::common::rng::CountingRng;

pub type TokioNetwork = FromTokio<TcpStream>;
pub type TestClient<'a> = MqttClient<'a, TokioNetwork, MAX_PROPERTIES, CountingRng>;

const BROKER: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 1883);
const USERNAME: &str = "test";
const PASSWORD: &str = "testPass";
const MAX_PROPERTIES: usize = 5;

static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

pub(crate) async fn connect_core<'a>(
    recv_buffer: &'a mut [u8],
    write_buffer: &'a mut [u8],
    client_id: Option<&'a str>,
    username: &'a str,
    password: &'a str,
) -> Result<TestClient<'a>, ReasonCode> {
    info!(
        "[CONNECT] Connecting to broker at {} with username {} and password {}",
        BROKER, username, password
    );

    let connection = TcpStream::connect(BROKER).await;

    if let Err(e) = connection {
        panic!(
            "[CONNECT] Error while connecting TCP session to {}: {}",
            BROKER, e
        );
    }

    let connection = TokioNetwork::new(connection.unwrap());

    let mut config = ClientConfig::new(MqttVersion::MQTTv5, CountingRng(20000));
    config.add_max_subscribe_qos(QualityOfService::QoS1);
    config.add_username(username);
    config.add_password(password);
    config.max_packet_size = 100;

    if let Some(client_id) = client_id {
        config.add_client_id(client_id);
    }

    let mut client = MqttClient::<TokioNetwork, MAX_PROPERTIES, CountingRng>::new(
        connection,
        write_buffer,
        recv_buffer,
        config,
    );

    client.connect_to_broker().await.map(|_| client)
}

pub async fn connected_client<'a>(
    recv_buffer: &'a mut [u8],
    write_buffer: &'a mut [u8],
    client_id: Option<&'a str>,
) -> TestClient<'a> {
    let result = connect_core(recv_buffer, write_buffer, client_id, USERNAME, PASSWORD).await;
    if let Err(e) = result {
        panic!("[CONNECT] MQTT handshake failed {}", e);
    } else {
        info!("[CONNECT] MQTT connection established");
    }
    result.unwrap()
}

pub async fn disconnect(client: &mut TestClient<'_>) {
    let result = client.disconnect().await;
    if let Err(e) = result {
        panic!("[DISCONNECT] MQTT Disconnect failed: {}", e);
    } else {
        info!("[DISCONNECT] Disconnected from broker");
    }
}

pub async fn subscribe(
    client: &mut TestClient<'_>,
    topic: &str,
    qos: QualityOfService,
    retain_handling: RetainHandling,
    retain_as_published: bool,
    no_local: bool,
) {
    let topic = Topic::new(topic)
        .quality_of_service(qos)
        .retain_handling(retain_handling)
        .retain_as_published(retain_as_published)
        .no_local(no_local);

    let result = client.subscribe_to_topic(topic).await;

    if let Err(e) = result {
        panic!("[SUBSCRIBE] Subscribing to topic {:?} failed: {}", topic, e);
    } else {
        info!("[SUBSCRIBE] Subscribed to topic {:?}", topic);
    }
}

pub async fn unsubscribe(client: &mut TestClient<'_>, topic: &str) {
    let result = client.unsubscribe_from_topic(topic).await;

    if let Err(e) = result {
        panic!("[UNSUBSCRIBE] Unsubscribing from topic {} failed: {}", topic, e);
    } else {
        info!("[UNSUBSCRIBE] Unsubscribed from topic {}", topic);
    }
}
