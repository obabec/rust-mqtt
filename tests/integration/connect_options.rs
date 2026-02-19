use std::{num::NonZero, time::Duration};

use rust_mqtt::{
    Bytes,
    client::{
        MqttError,
        event::Event,
        options::{PublicationOptions, SubscriptionOptions, TopicReference, WillOptions},
    },
    config::{KeepAlive, SessionExpiryInterval},
    types::{MqttBinary, MqttString, ReasonCode, TopicFilter, TopicName},
};
use tokio::{
    join,
    time::{sleep, timeout},
};
use tokio_test::assert_err;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, DEFAULT_QOS0_SUB_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::{assert_ok, assert_published, assert_recv, assert_recv_excl, assert_subscribe},
    utils::{connected_client, disconnect, unique_topic},
};

#[tokio::test]
#[test_log::test]
async fn maximum_packet_size_not_exceeded() {
    // Has to be a reasonable value not too close to 0, otherwise broker might not reply or something similar
    const MAX_PACKET_SIZE: u32 = 100;
    const PACKET_SIZE: usize = MAX_PACKET_SIZE as usize;

    // fixed header, topic name, packet identifier, property length
    const PAYLOAD_SIZE: usize = PACKET_SIZE - 2 - 3 - 2 - 1;

    let rx_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .maximum_packet_size(NonZero::new(MAX_PACKET_SIZE).unwrap());

    let topic_name = TopicName::new(MqttString::from_str("a").unwrap()).unwrap();
    let topic_filter = TopicFilter::from(topic_name.clone());

    let msg = [0u8; PAYLOAD_SIZE].as_slice();

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx = assert_ok!(connected_client(BROKER_ADDRESS, &rx_connect_options, None).await);

    let publisher = async {
        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            topic_filter.clone()
        );

        assert_recv_excl!(rx, topic_name);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "enable this once mosquitto v2.1.3 is used, see https://github.com/eclipse-mosquitto/mosquitto/issues/3503"]
#[tokio::test]
#[test_log::test]
async fn maximum_packet_size_barely_exceeded() {
    // Has to be a reasonable value not too close to 0, otherwise broker might not reply or something similar
    const MAX_PACKET_SIZE: u32 = 100;
    const PACKET_SIZE: usize = MAX_PACKET_SIZE as usize + 1;

    // fixed header, topic name, packet identifier, property length
    const PAYLOAD_SIZE: usize = PACKET_SIZE - 2 - 3 - 2 - 1;

    let rx_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .maximum_packet_size(NonZero::new(MAX_PACKET_SIZE).unwrap());

    let topic_name = TopicName::new(MqttString::from_str("b").unwrap()).unwrap();
    let topic_filter = TopicFilter::from(topic_name.clone());

    let msg = [0u8; PAYLOAD_SIZE].as_slice();

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx = assert_ok!(connected_client(BROKER_ADDRESS, &rx_connect_options, None).await);

    let publisher = async {
        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            topic_filter.clone()
        );

        assert_err!(
            timeout(Duration::from_secs(10), async {
                assert_recv!(rx);
            })
            .await,
            "Expected to receive nothing"
        );

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn maximum_packet_size_decently_exceeded() {
    const MAX_PACKET_SIZE: u32 = 5000;
    const PACKET_SIZE: usize = MAX_PACKET_SIZE as usize + 20;

    // fixed header, topic name, packet identifier, property length
    const PAYLOAD_SIZE: usize = PACKET_SIZE - 3 - 3 - 2 - 1;

    let rx_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .maximum_packet_size(NonZero::new(MAX_PACKET_SIZE).unwrap());

    let topic_name = TopicName::new(MqttString::from_str("c").unwrap()).unwrap();
    let topic_filter = TopicFilter::from(topic_name.clone());

    let msg = [0u8; PAYLOAD_SIZE].as_slice();

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx = assert_ok!(connected_client(BROKER_ADDRESS, &rx_connect_options, None).await);

    let publisher = async {
        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            topic_filter.clone()
        );

        assert_err!(
            timeout(Duration::from_secs(10), async {
                assert_recv!(rx);
            })
            .await,
            "Expected to receive nothing"
        );

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn maximum_packet_size_at_varbyteint_boundary_not_exceeded() {
    // Packets with length 130 don't exist, we have to craft one with length 129
    // Maximum packet size 129 or 130 leads to the same configuration in the client
    const MAX_PACKET_SIZE: u32 = 130;
    const PACKET_SIZE: usize = 129;

    // fixed header, topic name, packet identifier, property length
    const PAYLOAD_SIZE: usize = PACKET_SIZE - 2 - 3 - 2 - 1;

    let rx_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .maximum_packet_size(NonZero::new(MAX_PACKET_SIZE).unwrap());

    let topic_name = TopicName::new(MqttString::from_str("d").unwrap()).unwrap();
    let topic_filter = TopicFilter::from(topic_name.clone());

    let msg = [0u8; PAYLOAD_SIZE].as_slice();

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx = assert_ok!(connected_client(BROKER_ADDRESS, &rx_connect_options, None).await);

    let publisher = async {
        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            topic_filter.clone()
        );

        assert_recv_excl!(rx, topic_name);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn maximum_packet_size_at_varbyteint_boundary_exceeded() {
    // Packets with length 130 don't exist, we have to craft one with length 131
    const MAX_PACKET_SIZE: u32 = 130;
    const PACKET_SIZE: usize = 131;

    // fixed header, topic name, packet identifier, property length
    const PAYLOAD_SIZE: usize = PACKET_SIZE - 2 - 3 - 2 - 1;

    let rx_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .maximum_packet_size(NonZero::new(MAX_PACKET_SIZE).unwrap());

    let topic_name = TopicName::new(MqttString::from_str("e").unwrap()).unwrap();
    let topic_filter = TopicFilter::from(topic_name.clone());

    let msg = [0u8; PAYLOAD_SIZE].as_slice();

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx = assert_ok!(connected_client(BROKER_ADDRESS, &rx_connect_options, None).await);

    let publisher = async {
        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            topic_filter.clone()
        );

        assert_err!(
            timeout(Duration::from_secs(10), async {
                assert_recv!(rx);
            })
            .await,
            "Expected to receive nothing"
        );

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

// Keep alive timing test rules:
// - The server MUST NOT disconnect us before the keep alive interval has elapsed without a message
//   -> It can disconnect us earlier, so we can only reliably test with intervals < 1x keep alive
// - The server MUST disconnect us if it has not received any packet within 1.5x the keep alive interval
//   -> Some brokers behave not as strict, so we give it 2x the keep alive interval

#[tokio::test]
#[test_log::test]
async fn keep_alive_infinite() {
    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    assert_eq!(c.shared_config().keep_alive, KeepAlive::Infinite);

    assert_err!(
        timeout(Duration::from_secs(10), c.poll_header()).await,
        "expected to stay connected"
    );

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn keep_alive_via_ping() {
    let mut c = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &NO_SESSION_CONNECT_OPTIONS
                .clone()
                .keep_alive(KeepAlive::Seconds(NonZero::new(1).unwrap())),
            None
        )
        .await
    );
    assert_eq!(
        c.shared_config().keep_alive,
        KeepAlive::Seconds(NonZero::new(1).unwrap())
    );

    for _ in 0..10 {
        sleep(Duration::from_millis(950)).await;
        assert_ok!(c.ping().await);
        let event = assert_ok!(c.poll().await);
        assert!(matches!(event, Event::Pingresp));
    }

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn keep_alive_via_outgoing_publish() {
    let topic_name = unique_topic().0;
    let publish_options = PublicationOptions::new(TopicReference::Name(topic_name));

    let mut c = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &NO_SESSION_CONNECT_OPTIONS
                .clone()
                .keep_alive(KeepAlive::Seconds(NonZero::new(1).unwrap())),
            None
        )
        .await
    );
    assert_eq!(
        c.shared_config().keep_alive,
        KeepAlive::Seconds(NonZero::new(1).unwrap())
    );

    for _ in 0..10 {
        sleep(Duration::from_millis(950)).await;
        assert_ok!(c.publish(&publish_options, Bytes::from("")).await);
    }

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn keep_alive_via_incoming_qos1() {
    let (topic_name, topic_filter) = unique_topic();
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &NO_SESSION_CONNECT_OPTIONS
                .clone()
                .keep_alive(KeepAlive::Seconds(NonZero::new(2).unwrap())),
            None
        )
        .await
    );
    assert_eq!(
        rx.shared_config().keep_alive,
        KeepAlive::Seconds(NonZero::new(2).unwrap())
    );

    let publisher = async {
        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).at_least_once();

        for _ in 0..10 {
            sleep(Duration::from_secs(1)).await;
            assert_published!(tx, pub_options, Bytes::from(""));
        }

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS.at_least_once(), topic_filter);

        for _ in 0..10 {
            assert_recv_excl!(rx, topic_name);
        }

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn keep_alive_via_subscribe() {
    let topic_filter = unique_topic().1;
    let subscribe_options = SubscriptionOptions::new();

    let mut c = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &NO_SESSION_CONNECT_OPTIONS
                .clone()
                .keep_alive(KeepAlive::Seconds(NonZero::new(1).unwrap())),
            None
        )
        .await
    );
    assert_eq!(
        c.shared_config().keep_alive,
        KeepAlive::Seconds(NonZero::new(1).unwrap())
    );

    for _ in 0..10 {
        sleep(Duration::from_millis(950)).await;
        assert_ok!(
            c.subscribe(topic_filter.as_borrowed(), subscribe_options)
                .await
        );
        let event = assert_ok!(c.poll().await);
        assert!(matches!(event, Event::Suback(_)));
    }

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn keep_alive_not_kept_alive_idle_network() {
    let mut c = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &NO_SESSION_CONNECT_OPTIONS
                .clone()
                .keep_alive(KeepAlive::Seconds(NonZero::new(6).unwrap())),
            None
        )
        .await
    );
    assert_eq!(
        c.shared_config().keep_alive,
        KeepAlive::Seconds(NonZero::new(6).unwrap())
    );

    assert_err!(
        timeout(Duration::from_millis(5950), c.poll_header()).await,
        "expected to stay connected"
    );

    let e = assert_err!(assert_ok!(
        timeout(Duration::from_millis(6100), c.poll()).await,
        "expected to be disconnected"
    ));

    assert!(matches!(
        e,
        MqttError::Disconnect {
            reason: ReasonCode::KeepAliveTimeout,
            reason_string: _,
            server_reference: _,
        } | MqttError::Network(_)
    ));
}

#[tokio::test]
#[test_log::test]
async fn keep_alive_not_kept_alive_incoming_qos0() {
    let (topic_name, topic_filter) = unique_topic();
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &NO_SESSION_CONNECT_OPTIONS
                .clone()
                .keep_alive(KeepAlive::Seconds(NonZero::new(2).unwrap())),
            None
        )
        .await
    );
    assert_eq!(
        rx.shared_config().keep_alive,
        KeepAlive::Seconds(NonZero::new(2).unwrap())
    );

    let publisher = async {
        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).at_least_once();

        for _ in 0..10 {
            sleep(Duration::from_secs(1)).await;
            assert_published!(tx, pub_options, Bytes::from(""));
        }

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, topic_filter);

        assert_ok!(
            timeout(Duration::from_secs(4), async {
                loop {
                    if let Err(MqttError::Network(_)) = rx.poll().await {
                        break;
                    }
                }
            })
            .await,
            "expected to be disconnected"
        );
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn keep_alive_not_kept_alive_will_timing() {
    let (topic_name, topic_filter) = unique_topic();
    let will = WillOptions::new(topic_name.clone(), MqttBinary::from_slice(b"").unwrap())
        .delay_interval(3);
    let connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .will(will)
        .keep_alive(KeepAlive::Seconds(NonZero::new(3).unwrap()))
        .session_expiry_interval(SessionExpiryInterval::NeverEnd);

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, topic_filter);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &connect_options, None).await);

    assert_eq!(
        tx.shared_config().keep_alive,
        KeepAlive::Seconds(NonZero::new(3).unwrap())
    );

    assert_err!(
        timeout(Duration::from_millis(5950), rx.poll_header()).await,
        "expected to receive nothing"
    );

    assert_ok!(
        timeout(Duration::from_millis(3100), async {
            assert_recv_excl!(rx, topic_name);
        })
        .await,
        "expected to receive the will message"
    );

    let e = assert_err!(tx.poll().await);
    assert!(matches!(
        e,
        MqttError::Disconnect {
            reason: ReasonCode::KeepAliveTimeout,
            reason_string: _,
            server_reference: _,
        } | MqttError::Network(_)
    ));

    disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
}
