//! Short explanation of this example:
//! MQTT clients and servers are allowed to verify the payload for UTF-8 and can reject it, if the
//! payload format indicator claims that the payload is UTF-8 but it actually isn't. The client in
//! rust-mqtt does not implement this optional feature as it may not be necessary for some users,
//! or they might do the check themselves in order to convert payload to a `String` or `&str`.
//!
//! In this sketch, we want to implement this optional MQTT feature to reject packets, which
//! falsely claim UTF-8 payload. We craft these dishonest packets ourselves and assume that the
//! server forwards these without this optional check (which mosquitto in its default configuration
//! does). Additionally, we want to show how manual acknowledgements can be done for outgoing
//! publications as well (in this case, our dishonest packets).

use std::{
    net::{Ipv4Addr, SocketAddr},
    str::from_utf8,
    time::Duration,
};

use embedded_io_adapters::tokio_1::FromTokio;
use log::{error, info};
use rust_mqtt::{
    Bytes,
    buffer::*,
    client::{
        Client,
        event::{Event, Puback, Publish, Suback},
        options::{
            AckMode, AckOptions, ConnectOptions, DisconnectOptions, PublicationOptions,
            SubscriptionOptions, TopicReference,
        },
    },
    types::{IdentifiedQoS, MqttBinary, MqttString, ReasonCode, TopicFilter, TopicName},
};
use tokio::{net::TcpStream, select, time::sleep};
use tokio_test::assert_ok;

#[tokio::main]
async fn main() {
    env_logger::init();

    #[cfg(feature = "alloc")]
    let mut buffer = AllocBuffer;
    #[cfg(feature = "bump")]
    let mut buffer = [0; 1024];
    #[cfg(feature = "bump")]
    let mut buffer = BumpBuffer::new(&mut buffer);

    let mut client = Client::<'_, _, _, 1, 3, 3, 0, 16>::new(&mut buffer);

    // Acknowledge all packets manually which have a payload format indicator property with a value of
    // true (claiming that the payload is UTF-8). We intentionally leave the check for actual UTF-8
    // in the event handling code to showcase both positive and negative manual acknowledgements.
    client.ack_manually_when(&|publish| {
        publish
            .payload_format_indicator
            .is_some_and(|is_utf8| is_utf8)
    });

    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 1883);
    let connection = TcpStream::connect(addr).await.unwrap();
    let connection = FromTokio::new(connection);

    match client
        .connect(
            connection,
            &ConnectOptions::new()
                .user_name(MqttString::try_from("test").unwrap())
                .password(MqttBinary::try_from("testPass").unwrap())
                .clean_start(),
            None,
        )
        .await
    {
        Ok(c) => info!("Connected to server: {c:?}"),
        Err(e) => {
            error!("Failed to connect to server: {e:?}");
            return;
        }
    }

    let topic_string = MqttString::from_str("rust-mqtt/rocks").unwrap();
    let topic_filter = TopicFilter::new(topic_string.as_borrowed()).unwrap();
    let topic_name = TopicName::new(topic_string.as_borrowed()).unwrap();

    assert_ok!(
        client
            .subscribe(
                topic_filter.as_borrowed(),
                &SubscriptionOptions::new().exactly_once()
            )
            .await
    );

    match assert_ok!(client.poll().await) {
        Event::Suback(Suback {
            packet_identifier: _,
            reason_string: _,
            user_properties: _,
            reason_code,
        }) if reason_code.is_success() => {}
        _ => panic!("subscription failed"),
    }

    let valid_utf8 = Bytes::Borrowed("Hello World!".as_bytes());
    let invalid_utf8 = Bytes::Borrowed(&[0x80]);

    // Has payload format indicator and is valid UTF-8
    // => should be acknowledged automatically by our client
    //
    // Configured with `ack_manually`
    // => the outgoing publication must be acknowledged manually
    assert_ok!(
        client
            .publish(
                &PublicationOptions::new(TopicReference::Name(topic_name.as_borrowed()))
                    .payload_format_indicator(true)
                    .exactly_once()
                    .ack_manually(),
                valid_utf8.as_borrowed(),
            )
            .await
    );

    // Is valid UTF-8 but misses a payload format indicator
    // => should be acknowledged automatically by our client
    //
    // Not configured with `ack_manually` (outgoing QoS 1 publications can't be due to no outgoing acks)
    // => the outgoing publication is acknowledged automatically
    assert_ok!(
        client
            .publish(
                &PublicationOptions::new(TopicReference::Name(topic_name.as_borrowed()))
                    .at_least_once(),
                valid_utf8,
            )
            .await
    );

    // Has a payload format indicator of true but is invalid UTF-8
    // => should be acknowledged automatically when the client receives this publication again
    //
    // Configured with `ack_manually`
    // => the outgoing publication must be acknowledged manually
    assert_ok!(
        client
            .publish(
                &PublicationOptions::new(TopicReference::Name(topic_name))
                    .payload_format_indicator(true)
                    .exactly_once()
                    .ack_manually(),
                invalid_utf8,
            )
            .await
    );

    loop {
        select! {
            () = sleep(Duration::from_secs(5)) => {
                break;
            },
            header = client.poll_header() => {
                let h = assert_ok!(header);
                match assert_ok!(client.poll_body(h).await) {
                    // Outgoing publications & their acknowledgement counterpart
                    Event::PublishReceived(Puback { ack_mode: AckMode::Manual, packet_identifier, reason_code, reason_string, user_properties }) if reason_code.is_success() => {
                        info!("Manually releasing packet identifier {packet_identifier}");
                        client.manual_release(packet_identifier, &AckOptions::new().reason_string(MqttString::from_str("s").unwrap())).await.unwrap();
                    }

                    // Incoming publications & their acknowledgement counterpart
                    Event::Publish(Publish { ack_mode: AckMode::Manual, topic, payload_format_indicator, message, identified_qos, .. }) => {
                        // According to our predicate, matching this branch means that the packet claims UTF-8 status!
                        assert_eq!(payload_format_indicator, Some(true));

                        let ack_reason_code = if let Ok(message) = from_utf8(message.as_bytes()) {
                            // The packet correctly claimed UTF-8 status!
                            info!("Received valid publication: topic={topic:?}, payload_format_indicator={payload_format_indicator:?}, message={message}");

                            // We acknowledge the publication positively, all good!
                            ReasonCode::Success
                        } else {
                            // The packet incorrectly claimed UTF-8 status!
                            info!("Received invalid publication: topic={topic:?}, payload_format_indicator={payload_format_indicator:?},, message={:?}", message.as_bytes());

                            // We reject the publication as it is invalid.
                            ReasonCode::PayloadFormatInvalid
                        };

                        match identified_qos {
                            IdentifiedQoS::AtMostOnce => {}
                            IdentifiedQoS::AtLeastOnce(packet_identifier) => {
                                info!("Manually acknowledging packet identifier {packet_identifier}");
                                client.manual_acknowledge(packet_identifier, ack_reason_code, &AckOptions::new()).await.unwrap();
                            }
                            IdentifiedQoS::ExactlyOnce(packet_identifier) => {
                                info!("Manually receiving packet identifier {packet_identifier}");
                                client.manual_receive(packet_identifier, ack_reason_code, &AckOptions::new()).await.unwrap();
                            }
                        }
                    }

                    Event::Publish(Publish { ack_mode: AckMode::Automatic, topic, payload_format_indicator, message, .. }) => {
                        if let Ok(message) = from_utf8(message.as_bytes()) {
                            info!("Received publication: topic={topic:?}, payload_format_indicator={payload_format_indicator:?}, message={message}");
                        } else {
                            info!("Received publication: topic={topic:?}, payload_format_indicator={payload_format_indicator:?},, message={:?}", message.as_bytes());
                        }
                    }
                    Event::PublishReleased(Puback { ack_mode: AckMode::Manual, packet_identifier, .. }) => {
                        info!("Manually completing packet identifier {packet_identifier}");
                        client.manual_complete(packet_identifier, &AckOptions::new()).await.unwrap();
                    }
                    e => info!("Received {e:?}"),
                }
            }
        };
    }

    // For a correct TCP disconnection, one should make sure the underlying TCP socket
    // sends a FIN segment. However I could not get the tokio::TcpStream to behave that
    // way, so we just do nothing here. It's fine for MQTT operability realistically,
    // but for clean usage, the TCP should be closed properly.

    let _n = client.disconnect(&DisconnectOptions::new()).await.unwrap();
}
