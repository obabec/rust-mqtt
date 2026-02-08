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
        event::{Event, Puback, Suback},
        options::{
            ConnectOptions, DisconnectOptions, PublicationOptions, RetainHandling,
            SubscriptionOptions, TopicReference, WillOptions,
        },
    },
    config::{KeepAlive, SessionExpiryInterval},
    types::{MqttBinary, MqttString, QoS, TopicName, VarByteInt},
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

    let mut client = Client::<'_, _, _, 1, 1, 1, 1>::new(&mut buffer);

    let addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 1883);
    let connection = TcpStream::connect(addr).await.unwrap();
    let connection = FromTokio::new(connection);

    match client
        .connect(
            connection,
            &ConnectOptions::new()
                .clean_start()
                .session_expiry_interval(SessionExpiryInterval::Seconds(5))
                .keep_alive(KeepAlive::Seconds(5))
                .user_name(MqttString::try_from("test").unwrap())
                .password(MqttBinary::try_from("testPass").unwrap())
                .will(
                    WillOptions::new(
                        TopicName::new(MqttString::try_from("i/am/dead").unwrap()).unwrap(),
                        MqttBinary::try_from("Have a nice day!").unwrap(),
                    )
                    .exactly_once()
                    .retain()
                    .delay_interval(1)
                    .mark_payload_utf8()
                    .content_type(MqttString::try_from("txt").unwrap()),
                ),
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

    let subscription_identifier = if client.server_config().subscription_identifiers_supported {
        Some(VarByteInt::from(42u16))
    } else {
        None
    };

    let sub_options = SubscriptionOptions {
        retain_handling: RetainHandling::SendIfNotSubscribedBefore,
        retain_as_published: true,
        no_local: false,
        qos: QoS::ExactlyOnce,
        subscription_identifier,
    };

    let topic =
        TopicName::new(MqttString::from_str("rust-mqtt/is/great").unwrap()).unwrap();

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

    let pub_options =
        PublicationOptions::new(TopicReference::Mapping(topic.clone(), 1)).exactly_once();

    match client
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

    match client.poll().await {
        Ok(e) => info!("Received Event {:?}", e),
        Err(e) => {
            error!("Failed to poll: {:?}", e);
            return;
        }
    }

    match client.unsubscribe(topic.clone().into()).await {
        Ok(_) => info!("Sent Unsubscribe"),
        Err(e) => {
            error!("Failed to unsubscribe: {:?}", e);
            return;
        }
    };

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

    // Start a Quality of Service 2 publish flow
    let pub_options = PublicationOptions::new(TopicReference::Alias(1)).exactly_once();

    let incomplete_publish_packet_identifier = match client
        .publish(&pub_options, Bytes::from("something".as_bytes()))
        .await
    {
        Ok(pid) => {
            info!("Published to topic alias 1 aka \"rust-mqtt/is/great\"");
            pid
        }
        Err(e) => {
            error!("Failed to publish to topic alias {:?}", e);
            return;
        }
    };

    // Extract the session manually so we can simulate a dropped network connection.
    let session = client.session().clone();

    // Drop the client to simulate a lost network connection.
    drop(client);

    info!("Network connection dropped");

    // Wait at least 1 second so that server publishes our will.
    sleep(Duration::from_secs(2)).await;

    // The reason why a new bump buffer has to be created is because BufferProvider<'b> is also borrowed for 'b.
    // This is an inconvenience here but in reality it should be fine if the existing client is reused by reconnecting.
    #[cfg(feature = "bump")]
    let mut buffer = [0; 1024];
    #[cfg(feature = "bump")]
    let mut buffer = BumpBuffer::new(&mut buffer);

    // Continue the previous session
    let mut client = Client::<'_, _, _, 1, 1, 1, 1>::with_session(session, &mut buffer);

    let addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 1883);
    let connection = TcpStream::connect(addr).await.unwrap();
    let connection = FromTokio::new(connection);

    match client
        .connect(
            connection,
            &ConnectOptions::new()
                .user_name(MqttString::try_from("test").unwrap())
                .password(MqttBinary::try_from("testPass").unwrap()),
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

    let pub_options = PublicationOptions::new(TopicReference::Name(topic.clone())).exactly_once();

    match client
        .republish(
            incomplete_publish_packet_identifier,
            &pub_options,
            Bytes::from("something".as_bytes()),
        )
        .await
    {
        Ok(_) => info!(
            "Republished packet identifier {} after reconnecting",
            incomplete_publish_packet_identifier
        ),
        Err(e) => error!(
            "Failed to republish packet identifier {} due to {:?}",
            incomplete_publish_packet_identifier, e
        ),
    }

    loop {
        match client.poll().await {
            Ok(Event::PublishComplete(Puback {
                packet_identifier,
                reason_code,
            })) if packet_identifier == incomplete_publish_packet_identifier => {
                info!(
                    "Completed republish of packet identifier {}",
                    packet_identifier
                );
                break;
            }
            Ok(e) => info!("Received Event {:?}", e),
            Err(e) => {
                error!("Failed to poll: {:?}", e);
                return;
            }
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
