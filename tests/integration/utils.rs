use std::{
    net::{Ipv4Addr, SocketAddrV4}, sync::Once, time::Duration
};

use embedded_io_adapters::tokio_1::FromTokio;
use rust_mqtt::{
    client::{
        client::MqttClient,
        client_config::{ClientConfig, MqttVersion},
    },
    utils::{rng_generator::CountingRng, types::{QualityOfService, RetainHandling, Topic}},
};
use tokio::{net::TcpStream, time::{sleep, timeout}};
use tokio_test::{assert_err, assert_ok};

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

pub async fn connected_client<'a>(
    recv_buffer: &'a mut [u8],
    write_buffer: &'a mut [u8],
    client_id: Option<&'a str>,
) -> TestClient<'a> {
    let connection = TcpStream::connect(BROKER)
        .await
        .expect("Error while connecting over TCP to broker");

    let connection = TokioNetwork::new(connection);

    let mut config = ClientConfig::new(MqttVersion::MQTTv5, CountingRng(20000));
    config.add_max_subscribe_qos(QualityOfService::QoS1);
    config.add_username(USERNAME);
    config.add_password(PASSWORD);
    config.max_packet_size = 100;

    if let Some(client_id) = client_id {
        config.add_client_id(client_id);
    }

    let mut client = MqttClient::<TokioNetwork, 5, CountingRng>::new(
        connection,
        write_buffer,
        write_buffer.len(),
        recv_buffer,
        recv_buffer.len(),
        config,
    );

    client.connect_to_broker().await.expect("Error while connecting over MQTT to broker");

    client
}

pub async fn disconnect(
    client: &mut TestClient<'_>,
) {
    let result = client.disconnect().await;
    assert_ok!(result);
}

pub async fn publish(
    client: &mut TestClient<'_>,
    wait: u64,
    topic: &str,
    msg: &str,
    qos: QualityOfService,
    retain: bool,
    should_err: bool,
    can_err: bool,
) {
    sleep(Duration::from_secs(wait)).await;

    let result = client
        .send_message(topic, msg.as_bytes(), qos, retain)
        .await;

    if should_err {
        assert_err!(result);
    } else if can_err {

    } else {
        assert_ok!(result);
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
    let mut topic = Topic::new_with_default_options(topic, qos);
    topic.retain_handling = retain_handling;
    topic.retain_as_published = retain_as_published;
    topic.no_local = no_local;

    client.subscribe_to_topic(topic).await.expect("Error while subscribing");
}

pub async fn assert_receive(
    client: &mut TestClient<'_>,
    within: u64,
    topic: &str,
    msg: &str,
) {
    let duration = Duration::from_secs(within);

    let result = timeout(duration, client.receive_message()).await;

    let result = result.expect("Timeout while receiving");

    let (recv_topic, recv_msg) = result.expect("Error while receiving");
    assert_eq!(topic, recv_topic);
    assert_eq!(msg.as_bytes(), recv_msg);
}

pub async fn assert_no_receive(
    client: &mut TestClient<'_>,
    within: u64,
) {
    let duration = Duration::from_secs(within);

    timeout(duration, client.receive_message()).await.expect_err("Expected no event to come in");
}