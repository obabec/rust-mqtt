use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use embedded_io_adapters::tokio_1::FromTokio;
use log::{error, info};

use rust_mqtt::{
    Bytes,
    buffer::*,
    client::{
        Client,
        event::{Event, Suback},
        options::{
            ConnectOptions, DisconnectOptions, PublicationOptions, RetainHandling,
            SubscriptionOptions, WillOptions,
        },
    },
    config::{KeepAlive, SessionExpiryInterval},
    types::{MqttBinary, MqttString, QoS, TopicName},
};
use tokio::{net::TcpStream, select, time::sleep};

#[tokio::main]
async fn main() {
    env_logger::init();

    #[cfg(feature = "alloc")]
    let mut buffer = AllocBuffer;
    #[cfg(feature = "bump")]
    let mut buffer = [0; 1024];
    #[cfg(feature = "bump")]
    let mut buffer = BumpBuffer::new(&mut buffer);

    let mut client = Client::<'_, _, _, 1, 1, 1>::new(&mut buffer);

    let addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 1883);
    let connection = TcpStream::connect(addr).await.unwrap();
    let connection = FromTokio::new(connection);

    match client
        .connect(
            connection,
            &ConnectOptions {
                session_expiry_interval: SessionExpiryInterval::Seconds(5),
                clean_start: false,
                keep_alive: KeepAlive::Seconds(3),
                will: Some(WillOptions {
                    will_qos: QoS::ExactlyOnce,
                    will_retain: true,
                    will_topic: MqttString::try_from("dead").unwrap(),
                    will_payload: MqttBinary::try_from("joe mama").unwrap(),
                    will_delay_interval: 10,
                    is_payload_utf8: true,
                    message_expiry_interval: Some(20),
                    content_type: Some(MqttString::try_from("txt").unwrap()),
                    response_topic: None,
                    correlation_data: None,
                }),
                user_name: Some(MqttString::try_from("test").unwrap()),
                password: Some(MqttBinary::try_from("testPass").unwrap()),
            },
            Some(MqttString::try_from("rust-mqtt-demo-client").unwrap()),
        )
        .await
    {
        Ok(c) => {
            info!("Connected to server: {:?}", c);
            info!("{:?}", client.client_config());
            info!("{:?}", client.server_config());
            info!("{:?}", client.shared_config());
            info!("{:?}", client.session());
        }
        Err(e) => {
            error!("Failed to connect to server: {:?}", e);
            return;
        }
    };

    #[cfg(feature = "bump")]
    unsafe {
        client.buffer().reset()
    };

    let sub_options = SubscriptionOptions {
        retain_handling: RetainHandling::SendIfNotSubscribedBefore,
        retain_as_published: true,
        no_local: false,
        qos: QoS::ExactlyOnce,
    };

    let topic =
        unsafe { TopicName::new_unchecked(MqttString::from_slice("rust-mqtt/is/great").unwrap()) };

    match client.subscribe(topic.clone().into(), sub_options).await {
        Ok(_) => info!("Sent Subscribe"),
        Err(e) => {
            error!("Failed to subscribe: {:?}", e);
            return;
        }
    };

    match client.poll().await {
        Ok(Event::Suback(Suback {
            packet_identifier: _,
            reason_code,
        })) => info!("Subscribed with reason code {:?}", reason_code),
        Ok(e) => {
            error!("Expected Suback but received event {:?}", e);
            return;
        }
        Err(e) => {
            error!("Failed to receive Suback {:?}", e);
            return;
        }
    }

    let pub_options = PublicationOptions {
        retain: false,
        topic: topic.clone(),
        qos: QoS::ExactlyOnce,
    };

    let publish_packet_id = match client
        .publish(&pub_options, Bytes::from("anything".as_bytes()))
        .await
    {
        Ok(i) => {
            info!("Published message with packet identifier {}", i);
            i
        }
        Err(e) => {
            error!("Failed to send Publish {:?}", e);
            return;
        }
    };

    let pub_options = PublicationOptions {
        retain: false,
        topic: topic.clone(),
        qos: QoS::ExactlyOnce,
    };
    client
        .republish(
            publish_packet_id,
            &pub_options,
            Bytes::from("anything".as_bytes()),
        )
        .await
        .unwrap();

    loop {
        match client.poll().await {
            Ok(Event::PublishComplete(_)) => {
                info!("Publish complete");
                break;
            }
            Ok(e) => info!("Received event {:?}", e),
            Err(e) => {
                error!("Failed to poll: {:?}", e);
                return;
            }
        }
    }

    let mut pings = 3;

    while pings > 0 {
        select! {
            _ = sleep(Duration::from_secs(4)) => {
                match client.ping().await {
                    Ok(_) => {
                        pings -= 1;
                        info!("Pinged server");
                    },
                    Err(e) => {
                        error!("Failed to ping: {:?}", e);
                        return;
                    }
                }
            },
            header = client.poll_header() => {
                let h = match header {
                    Ok(h) => h,
                    Err(e) => {
                        error!("Failed to poll header: {:?}", e);
                        return;
                    }
                };
                info!("Received header {:?}", h.packet_type());
                match client.poll_body(h).await {
                    Ok(e) => info!("Received Event {:?}", e),
                    Err(e) => {
                        error!("Failed to poll body: {:?}", e);
                        return;
                    }
                }
            }
        };
    }

    match client.unsubscribe(topic.into()).await {
        Ok(_) => info!("Sent Unsubscribe"),
        Err(e) => {
            error!("Failed to unsubscribe: {:?}", e);
            return;
        }
    };

    match client.poll().await {
        Ok(e) => info!("Received Event {:?}", e),
        Err(e) => {
            error!("Failed to poll: {:?}", e);
            return;
        }
    }

    match client.poll().await {
        Ok(Event::Unsuback(Suback {
            packet_identifier: _,
            reason_code,
        })) => info!("Unsubscribed with reason code {:?}", reason_code),
        Ok(e) => {
            info!("Expected Unsuback but received event {:?}", e);
            return;
        }
        Err(e) => {
            error!("Failed to receive Unsuback {:?}", e);
            return;
        }
    }

    match client
        .disconnect(&DisconnectOptions {
            publish_will: false,
            session_expiry_interval: None,
        })
        .await
    {
        Ok(_) => info!("Disconnected from server"),
        Err(e) => {
            error!("Failed to disconnect from server: {:?}", e);
            return;
        }
    }
}
