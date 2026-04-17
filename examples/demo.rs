use std::{
    net::{Ipv4Addr, SocketAddr},
    num::NonZero,
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
            SubscriptionOptions, TopicReference, UnsubscriptionOptions, WillOptions,
        },
    },
    config::{KeepAlive, SessionExpiryInterval},
    types::{MqttBinary, MqttString, TopicName, VarByteInt},
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

    let mut client = Client::<'_, _, _, 1, 1, 1, 1, 16>::new(&mut buffer);

    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 1883);
    let connection = TcpStream::connect(addr).await.unwrap();
    let connection = FromTokio::new(connection);

    match client
        .connect(
            connection,
            &ConnectOptions::new()
                .clean_start()
                .session_expiry_interval(SessionExpiryInterval::Seconds(5))
                .keep_alive(KeepAlive::Seconds(NonZero::new(5).unwrap()))
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
                    .payload_format_indicator(true)
                    .content_type(MqttString::try_from("txt").unwrap()),
                ),
            Some(MqttString::try_from("rust-mqtt-demo-client").unwrap()),
        )
        .await
    {
        Ok(c) => {
            info!("Connected to server: {c:?}");
            info!("{:?}", client.client_config());
            info!("{:?}", client.server_config());
            info!("{:?}", client.shared_config());
            info!("{:?}", client.session());
        }
        Err(e) => {
            error!("Failed to connect to server: {e:?}");
            return;
        }
    }

    #[cfg(feature = "bump")]
    unsafe {
        client.buffer_mut().reset();
    }

    let mut sub_options = SubscriptionOptions::new()
        .retain_handling(RetainHandling::SendIfNotSubscribedBefore)
        .retain_as_published()
        .exactly_once();

    if client.server_config().subscription_identifiers_supported {
        sub_options.subscription_identifier = Some(VarByteInt::from(42u16));
    }

    let topic = TopicName::new(MqttString::from_str("rust-mqtt/is/great").unwrap()).unwrap();

    match client.subscribe(topic.clone().into(), &sub_options).await {
        Ok(_) => info!("Sent Subscribe"),
        Err(e) => {
            error!("Failed to subscribe: {e:?}");
            return;
        }
    }

    match client.poll().await {
        Ok(Event::Suback(Suback {
            packet_identifier: _,
            reason_string: _,
            user_properties: _,
            reason_code,
        })) => {
            info!("Subscribed with reason code {reason_code:?}")
        }
        Ok(e) => {
            error!("Expected Suback but received event {e:?}");
            return;
        }
        Err(e) => {
            error!("Failed to receive Suback {e:?}");
            return;
        }
    }

    let pub_options = PublicationOptions::new(TopicReference::Mapping(
        topic.clone(),
        NonZero::new(1).unwrap(),
    ))
    .exactly_once();

    match client
        .publish(&pub_options, Bytes::from("anything".as_bytes()))
        .await
    {
        Ok(i) => {
            info!("Published message with packet identifier {}", i.unwrap());
            i.unwrap()
        }
        Err(e) => {
            error!("Failed to send Publish {e:?}");
            return;
        }
    };

    loop {
        match client.poll().await {
            Ok(Event::PublishComplete(_)) => {
                info!("Publish complete");
                break;
            }
            Ok(e) => info!("Received event {e:?}"),
            Err(e) => {
                error!("Failed to poll: {e:?}");
                return;
            }
        }
    }

    let mut pings = 3;

    while pings > 0 {
        select! {
            () = sleep(Duration::from_secs(4)) => {
                match client.ping().await {
                    Ok(()) => {
                        pings -= 1;
                        info!("Pinged server");
                    },
                    Err(e) => {
                        error!("Failed to ping: {e:?}");
                        return;
                    }
                }
            },
            header = client.poll_header() => {
                let h = match header {
                    Ok(h) => h,
                    Err(e) => {
                        error!("Failed to poll header: {e:?}");
                        return;
                    }
                };
                info!("Received header {:?}", h.packet_type());
                match client.poll_body(h).await {
                    Ok(e) => info!("Received Event {e:?}"),
                    Err(e) => {
                        error!("Failed to poll body: {e:?}");
                        return;
                    }
                }
            }
        };
    }

    match client.poll().await {
        Ok(e) => info!("Received Event {e:?}"),
        Err(e) => {
            error!("Failed to poll: {e:?}");
            return;
        }
    }

    match client
        .unsubscribe(topic.clone().into(), &UnsubscriptionOptions::new())
        .await
    {
        Ok(_) => info!("Sent Unsubscribe"),
        Err(e) => {
            error!("Failed to unsubscribe: {e:?}");
            return;
        }
    }

    match client.poll().await {
        Ok(Event::Unsuback(Suback {
            packet_identifier: _,
            reason_string: _,
            user_properties: _,
            reason_code,
        })) => {
            info!("Unsubscribed with reason code {reason_code:?}")
        }
        Ok(e) => {
            info!("Expected Unsuback but received event {e:?}");
            return;
        }
        Err(e) => {
            error!("Failed to receive Unsuback {e:?}");
            return;
        }
    }

    // Start a Quality of Service 2 publish flow
    let pub_options =
        PublicationOptions::new(TopicReference::Alias(NonZero::new(1).unwrap())).exactly_once();

    let incomplete_publish_packet_identifier = match client
        .publish(&pub_options, Bytes::from("something".as_bytes()))
        .await
    {
        Ok(pid) => {
            info!("Published to topic alias 1 aka \"rust-mqtt/is/great\"");
            pid.unwrap()
        }
        Err(e) => {
            error!("Failed to publish to topic alias {e:?}");
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
    let mut client = Client::<'_, _, _, 1, 1, 1, 1, 16>::with_session(session, &mut buffer);

    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 1883);
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
            info!("Connected to server: {c:?}");
            info!("{:?}", client.client_config());
            info!("{:?}", client.server_config());
            info!("{:?}", client.shared_config());
            info!("{:?}", client.session());
        }
        Err(e) => {
            error!("Failed to connect to server: {e:?}");
            return;
        }
    }

    let pub_options = PublicationOptions::new(TopicReference::Name(topic.clone())).exactly_once();

    match client
        .republish(
            incomplete_publish_packet_identifier,
            &pub_options,
            Bytes::from("something".as_bytes()),
        )
        .await
    {
        Ok(()) => info!(
            "Republished packet identifier {incomplete_publish_packet_identifier} after reconnecting"
        ),
        Err(e) => error!(
            "Failed to republish packet identifier {incomplete_publish_packet_identifier} due to {e:?}"
        ),
    }

    loop {
        match client.poll().await {
            Ok(Event::PublishComplete(Puback {
                packet_identifier,
                reason_code: _,
                reason_string: _,
                user_properties: _,
            })) if packet_identifier == incomplete_publish_packet_identifier => {
                info!("Completed republish of packet identifier {packet_identifier}");
                break;
            }
            Ok(e) => info!("Received Event {e:?}"),
            Err(e) => {
                error!("Failed to poll: {e:?}");
                return;
            }
        }
    }

    match client.disconnect(&DisconnectOptions::new()).await {
        Ok(()) => info!("Disconnected from server"),
        Err(e) => {
            error!("Failed to disconnect from server: {e:?}");
            return;
        }
    }
}
