use std::time::Duration;

use rust_mqtt::{
    client::options::{PublicationOptions, TopicReference},
    types::{MqttString, TopicFilter, TopicName},
};
use tokio::{
    join,
    time::{sleep, timeout},
};
use tokio_test::assert_err;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, DEFAULT_QOS0_SUB_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::{assert_ok, assert_published, assert_recv, assert_recv_excl, assert_subscribe},
    utils::{connected_client, disconnect},
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
        .maximum_packet_size(MAX_PACKET_SIZE);

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
        .maximum_packet_size(MAX_PACKET_SIZE);

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
        .maximum_packet_size(MAX_PACKET_SIZE);

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
        .maximum_packet_size(MAX_PACKET_SIZE);

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
        .maximum_packet_size(MAX_PACKET_SIZE);

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
