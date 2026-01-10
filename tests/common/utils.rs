use std::{net::SocketAddr, sync::Mutex};

use embedded_io_adapters::tokio_1::FromTokio;
use log::{info, warn};
use rust_mqtt::{
    Bytes,
    buffer::AllocBuffer,
    client::{
        Client, MqttError,
        event::{Event, Publish, Suback},
        options::{ConnectOptions, DisconnectOptions, PublicationOptions, SubscriptionOptions},
    },
    types::{IdentifiedQoS, MqttString, QoS, ReasonCode, TopicFilter, TopicName},
};
use tokio::net::TcpStream;

use crate::common::{
    FailingClient, TestClient,
    failing::{ByteLimit, FailingTcp},
    fmt::warn_inspect,
};

static UNIQUE_TOPIC_COUNTER: Mutex<u64> = Mutex::new(0);
pub static ALLOC: StaticAlloc = StaticAlloc(AllocBuffer);

fn unique_number() -> u64 {
    let mut counter = UNIQUE_TOPIC_COUNTER.lock().unwrap();
    let number = *counter;
    *counter += 1;

    number
}
pub fn unique_topic() -> (TopicName<'static>, TopicFilter<'static>) {
    let s = format!("rust/mqtt/is-amazing/{}", unique_number());
    let b = s.into_bytes().into_boxed_slice();
    let s = MqttString::new(Bytes::Owned(b)).unwrap();

    let n = TopicName::new(s).unwrap();
    let f = n.clone().into();

    (n, f)
}

pub struct StaticAlloc(AllocBuffer);
impl StaticAlloc {
    pub fn get(&self) -> &'static mut AllocBuffer {
        let inner = &raw const self.0 as *mut AllocBuffer;
        unsafe { &mut *inner }
    }
}

pub async fn tcp_connection_raw(broker: SocketAddr) -> Result<TcpStream, MqttError<'static>> {
    warn_inspect!(
        TcpStream::connect(broker).await,
        "Error while connecting TCP session"
    )
    .map_err(|_| MqttError::RecoveryRequired)
}

pub async fn tcp_connection(
    broker: SocketAddr,
) -> Result<FromTokio<TcpStream>, MqttError<'static>> {
    tcp_connection_raw(broker).await.map(FromTokio::new)
}

pub async fn connected_client(
    broker: SocketAddr,
    options: &ConnectOptions<'static>,
    client_identifier: Option<MqttString<'_>>,
) -> Result<TestClient<'static>, MqttError<'static>> {
    let mut client = Client::new(ALLOC.get());

    let tcp = tcp_connection(broker).await?;

    warn_inspect!(
        client.connect(tcp, options, client_identifier).await,
        "Client::connect() failed"
    )
    .map(|_| client)
}

pub async fn connected_failing_client(
    broker: SocketAddr,
    options: &ConnectOptions<'static>,
    client_identifier: Option<MqttString<'_>>,
    read_limit: ByteLimit,
    write_limit: ByteLimit,
) -> Result<FailingClient<'static>, MqttError<'static>> {
    let mut client = Client::new(ALLOC.get());

    let tcp = tcp_connection_raw(broker).await?;
    let tcp = FailingTcp::new(tcp, read_limit, write_limit);
    let tcp = FromTokio::new(tcp);

    warn_inspect!(
        client.connect(tcp, options, client_identifier).await,
        "Client::connect() failed"
    )
    .map(|_| client)
}

pub async fn disconnect(client: &mut TestClient<'_>, options: &DisconnectOptions) {
    let _ = warn_inspect!(
        client.disconnect(options).await,
        "Client::disconnect() failed"
    );
}

pub async fn subscribe<'c>(
    client: &mut TestClient<'c>,
    options: SubscriptionOptions,
    topic: TopicFilter<'_>,
) -> Result<QoS, MqttError<'c>> {
    let pid = warn_inspect!(
        client.subscribe(topic, options).await,
        "Client::subscribe() failed"
    )?;

    loop {
        match warn_inspect!(client.poll().await, "Client::poll() failed")? {
            Event::Suback(Suback {
                packet_identifier,
                reason_code,
            }) if packet_identifier == pid => {
                info!("Subscribed with reason code {:?}", reason_code);
                break match reason_code {
                    ReasonCode::Success => Ok(QoS::AtMostOnce),
                    ReasonCode::GrantedQoS1 => Ok(QoS::AtLeastOnce),
                    ReasonCode::GrantedQoS2 => Ok(QoS::ExactlyOnce),
                    _ => unreachable!(),
                };
            }
            Event::Suback(Suback {
                packet_identifier,
                reason_code: _,
            }) => warn!(
                "Expected SUBACK for packet identifier {}, but received SUBACK for packet identifier {}",
                pid, packet_identifier
            ),
            e => warn!("Expected Event::Suback, but received {:?}", e),
        }
    }
}

pub async fn unsubscribe<'c>(
    client: &mut TestClient<'c>,
    topic: TopicFilter<'_>,
) -> Result<(), MqttError<'c>> {
    let pid = warn_inspect!(
        client.unsubscribe(topic).await,
        "Client::unsubscribe() failed"
    )?;

    loop {
        match warn_inspect!(client.poll().await, "Client::poll() failed")? {
            Event::Unsuback(Suback {
                packet_identifier,
                reason_code,
            }) if packet_identifier == pid => {
                info!("Unsubscribed with reason code {:?}", reason_code);
                break Ok(());
            }
            Event::Unsuback(Suback {
                packet_identifier,
                reason_code: _,
            }) => warn!(
                "Expected UNSUBACK for packet identifier {}, but received UNSUBACK for packet identifier {}",
                pid, packet_identifier
            ),
            e => warn!("Expected Event::Unsuback, but received {:?}", e),
        }
    }
}

pub async fn receive_publish<'c>(
    client: &mut TestClient<'c>,
) -> Result<Publish<'c, 1>, MqttError<'c>> {
    loop {
        match warn_inspect!(client.poll().await, "Client::poll() failed")? {
            Event::Publish(p) => break Ok(p),
            e => warn!("Expected PUBLISH, but received {:?}", e),
        }
    }
}

pub async fn receive_and_complete<'c>(
    client: &mut TestClient<'c>,
) -> Result<Publish<'c, 1>, MqttError<'c>> {
    let publish = receive_publish(client).await?;

    match publish.identified_qos {
        IdentifiedQoS::AtMostOnce => {}
        IdentifiedQoS::AtLeastOnce(_) => {}
        IdentifiedQoS::ExactlyOnce(pid) => loop {
            match warn_inspect!(client.poll().await, "Client::poll() failed")? {
                Event::PublishReleased(p) if p.packet_identifier == pid => {
                    break;
                }
                Event::PublishReleased(p) => warn!(
                    "Expected PUBREL with packet identifier {}, but received PUBREL with packet identifier {}",
                    pid, p.packet_identifier
                ),
                e => warn!("Expected PUBLISH, but received {:?}", e),
            }
        },
    }

    Ok(publish)
}

pub async fn publish_and_complete<'c>(
    client: &mut TestClient<'c>,
    options: &PublicationOptions<'_>,
    message: Bytes<'_>,
) -> Result<u16, MqttError<'c>> {
    let pid = warn_inspect!(
        client.publish(options, message).await,
        "Client::poll() failed"
    )?;

    match options.qos {
        QoS::AtMostOnce => {}
        QoS::AtLeastOnce => loop {
            match warn_inspect!(client.poll().await, "Client::poll() failed")? {
                Event::PublishAcknowledged(p) if p.packet_identifier == pid => break,
                Event::PublishAcknowledged(p) => warn!(
                    "Expected PUBACK with packet identifier {}, but received PUBACK with packet identifier {}",
                    pid, p.packet_identifier
                ),
                e => warn!("Expected PUBACK, but received {:?}", e),
            }
        },
        QoS::ExactlyOnce => {
            loop {
                match warn_inspect!(client.poll().await, "Client::poll() failed")? {
                    Event::PublishReceived(p) if p.packet_identifier == pid => break,
                    Event::PublishReceived(p) => warn!(
                        "Expected PUBREC with packet identifier {}, but received PUBREC with packet identifier {}",
                        pid, p.packet_identifier
                    ),
                    e => warn!("Expected PUBREC, but received {:?}", e),
                }
            }
            loop {
                match warn_inspect!(client.poll().await, "Client::poll() failed")? {
                    Event::PublishComplete(p) if p.packet_identifier == pid => break,
                    Event::PublishComplete(p) => warn!(
                        "Expected PUBCOMP with packet identifier {}, but received PUBCOMP with packet identifier {}",
                        pid, p.packet_identifier
                    ),
                    e => warn!("Expected PUBCOMP, but received {:?}", e),
                }
            }
        }
    }

    Ok(pid)
}
