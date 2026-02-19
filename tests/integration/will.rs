use std::time::Duration;

use rust_mqtt::{
    client::{
        MqttError,
        event::Publish,
        options::{DisconnectOptions, WillOptions},
    },
    config::SessionExpiryInterval,
    types::{IdentifiedQoS, MqttBinary, MqttString, ReasonCode},
};
use tokio::{
    join,
    time::{sleep, timeout},
};
use tokio_test::assert_err;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, DEFAULT_QOS0_SUB_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::{assert_ok, assert_recv, assert_recv_excl, assert_subscribe},
    fmt::warn_inspect,
    utils::{connected_client, disconnect, receive_and_complete, tcp_connection, unique_topic},
};

#[tokio::test]
#[test_log::test]
async fn network_failure() {
    let (will_topic_name, will_topic_filter) = unique_topic();

    let will_msg = "I am prepared to meet my maker. Whether my maker is prepared for the great ordeal of meeting me is another matter.";
    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    );

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        drop(tx);
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter);

        let Publish {
            dup,
            identified_qos,
            retain,
            topic: _,
            payload_format_indicator,
            message_expiry_interval,
            response_topic,
            correlation_data,
            subscription_identifiers,
            content_type,
            message,
        } = assert_recv_excl!(rx, will_topic_name);

        assert!(!dup);
        assert_eq!(identified_qos, IdentifiedQoS::AtMostOnce);
        assert!(!retain);
        assert_eq!(payload_format_indicator, None);
        assert_eq!(message_expiry_interval, None);
        assert_eq!(response_topic, None);
        assert_eq!(correlation_data, None);
        assert!(subscription_identifiers.is_empty());
        assert_eq!(content_type, None);
        assert_eq!(&*message, will_msg.as_bytes());

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn disconnect_with_will_message() {
    let (will_topic_name, will_topic_filter) = unique_topic();

    let will_msg = "The trouble with quotes about death is that 99.9% of them are made by people who are still alive.";
    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    );

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        assert_ok!(tx.disconnect(DEFAULT_DC_OPTIONS).await);
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter);

        let Publish {
            dup,
            identified_qos,
            retain,
            topic: _,
            payload_format_indicator,
            message_expiry_interval,
            response_topic,
            correlation_data,
            subscription_identifiers,
            content_type,
            message,
        } = assert_recv_excl!(rx, will_topic_name);

        assert!(!dup);
        assert_eq!(identified_qos, IdentifiedQoS::AtMostOnce);
        assert!(!retain);
        assert_eq!(payload_format_indicator, None);
        assert_eq!(message_expiry_interval, None);
        assert_eq!(response_topic, None);
        assert_eq!(correlation_data, None);
        assert!(subscription_identifiers.is_empty());
        assert_eq!(content_type, None);
        assert_eq!(&*message, will_msg.as_bytes());

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn disconnect_with_will_message_recovered() {
    let (will_topic_name, will_topic_filter) = unique_topic();

    let id = MqttString::from_str("WILL_DISCONNECT_WITH_WILL_MESSAGE_RECOVERED").unwrap();

    let will_msg = "Immortality . . . a fate worse than death.";
    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(10);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .session_expiry_interval(SessionExpiryInterval::Seconds(10))
        .will(will);

    let mut tx = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &will_connect_options,
            Some(id.as_borrowed())
        )
        .await
    );
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        assert_ok!(tx.disconnect(DEFAULT_DC_OPTIONS).await);

        sleep(Duration::from_secs(1)).await;

        let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);

        let mut will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        will_connect_options.clean_start = false;

        let info = assert_ok!(warn_inspect!(
            tx.connect(tcp, &will_connect_options, Some(id.as_borrowed()))
                .await,
            "Client::connect() failed"
        ));
        assert!(info.session_present);

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter);

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
async fn normal_disconnection() {
    let (will_topic_name, will_topic_filter) = unique_topic();

    let will_msg = "I'm not afraid to die, I just don't want to be there when it happens.";
    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    );

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        assert_ok!(tx.disconnect(&DisconnectOptions::new()).await);
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter);

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
async fn properties() {
    let (will_topic_name, will_topic_filter) = unique_topic();

    let will_msg = "The trouble with quotes about death is that 99.9% of them are made by people who are still alive.";
    let will_content_type = MqttString::from_str("application/json").unwrap();
    let will_correlation_data = MqttBinary::from_slice(b"ask3n028cnc+wscuw c09qn").unwrap();
    let will_response_topic = unique_topic().0;

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .content_type(will_content_type.clone())
    .correlation_data(will_correlation_data.clone())
    .payload_format_indicator(true)
    .message_expiry_interval(1234)
    .response_topic(will_response_topic.clone());

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        assert_ok!(tx.disconnect(DEFAULT_DC_OPTIONS).await);
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter);

        let Publish {
            dup,
            identified_qos,
            retain,
            topic: _,
            payload_format_indicator,
            message_expiry_interval,
            response_topic,
            correlation_data,
            subscription_identifiers,
            content_type,
            message,
        } = assert_recv_excl!(rx, will_topic_name);

        assert!(!dup);
        assert_eq!(identified_qos, IdentifiedQoS::AtMostOnce);
        assert!(!retain);
        assert_eq!(payload_format_indicator, Some(true));
        assert_eq!(message_expiry_interval, Some(1234));
        assert_eq!(response_topic, Some(will_response_topic));
        assert_eq!(correlation_data, Some(will_correlation_data));
        assert!(subscription_identifiers.is_empty());
        assert_eq!(content_type, Some(will_content_type));
        assert_eq!(&*message, will_msg.as_bytes());

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn qos0() {
    let (will_topic_name, will_topic_filter) = unique_topic();

    let will_msg = "I do not fear death. I had been dead for billions of years before I was born and had not suffered the slightest inconvenience.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    );

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        assert_ok!(tx.disconnect(DEFAULT_DC_OPTIONS).await);
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            will_topic_filter
        );

        let Publish { identified_qos, .. } = assert_recv_excl!(rx, will_topic_name);

        assert_eq!(identified_qos, IdentifiedQoS::AtMostOnce);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn qos1() {
    let (will_topic_name, will_topic_filter) = unique_topic();

    let will_msg = "Death is not the opposite of life but a part of it";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .at_least_once();

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        assert_ok!(tx.disconnect(DEFAULT_DC_OPTIONS).await);
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            will_topic_filter
        );

        let Publish { identified_qos, .. } = assert_recv_excl!(rx, will_topic_name);

        assert!(matches!(identified_qos, IdentifiedQoS::AtLeastOnce(_)));

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn qos2() {
    let (will_topic_name, will_topic_filter) = unique_topic();

    let will_msg = "Death smiles at us all; all we can do is smile back.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .exactly_once();

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        assert_ok!(tx.disconnect(DEFAULT_DC_OPTIONS).await);
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            will_topic_filter
        );

        let Publish { identified_qos, .. } = assert_recv_excl!(rx, will_topic_name);

        assert!(matches!(identified_qos, IdentifiedQoS::ExactlyOnce(_)));

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn message_expiry_interval_basic() {
    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "The life of the dead is placed in the memory of the living.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .message_expiry_interval(10);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(5)).await;
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        let Publish {
            message_expiry_interval,
            message,
            ..
        } = assert_recv_excl!(rx, will_topic_name);

        assert_eq!(&*message, will_msg.as_bytes());
        assert!(message_expiry_interval.is_some());
        assert!(matches!(message_expiry_interval.unwrap(), 9..=10));
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn message_expiry_interval_completely_expired() {
    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "Life is pleasant. Death is peaceful. It's the transition that's troublesome.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .message_expiry_interval(5);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        sleep(Duration::from_secs(6)).await;

        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        assert_err!(
            timeout(Duration::from_secs(4), async {
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
async fn will_delay_interval_basic() {
    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "Death is nature's way of saying, 'Your table is ready.'";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(5);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .session_expiry_interval(SessionExpiryInterval::NeverEnd)
        .will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        assert_err!(
            timeout(Duration::from_secs(5), async {
                assert_recv!(rx);
            })
            .await,
            "Expected to receive nothing"
        );

        let Publish { message, .. } = assert_recv_excl!(rx, will_topic_name);

        assert_eq!(&*message, will_msg.as_bytes());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "enable this test once the fix of https://github.com/eclipse-mosquitto/mosquitto/issues/3505 (mosquitto v2.1.3) is available"]
#[tokio::test]
#[test_log::test]
async fn session_expires_right_after_disconnect() {
    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "Death may be the greatest of all human blessings";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(100);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        let Publish { topic, message, .. } = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(2), receive_and_complete(&mut rx)).await
        ));

        assert_eq!(topic, will_topic_name);
        assert_eq!(&*message, will_msg.as_bytes());

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn session_expires_before_will_delay_interval() {
    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "To die will be an awfully big adventure.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(100);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .session_expiry_interval(SessionExpiryInterval::Seconds(5))
        .will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        assert_err!(
            timeout(Duration::from_secs(5), async {
                assert_recv!(rx);
            })
            .await,
            "Expected to receive nothing"
        );

        let Publish { topic, message, .. } = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(2), receive_and_complete(&mut rx)).await
        ));

        assert_eq!(topic, will_topic_name);
        assert_eq!(&*message, will_msg.as_bytes());

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn will_delay_interval_completely_expired() {
    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "The song is ended, but the melody lingers on.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(5);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        sleep(Duration::from_secs(7)).await;

        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        assert_err!(
            timeout(Duration::from_secs(3), async {
                assert_recv!(rx);
            })
            .await,
            "Expected to receive nothing"
        );
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

// fails with mosquitto v2.1.2
#[ignore = "enable this test once mosquitto v2.1.3 is used"]
#[tokio::test]
#[test_log::test]
async fn clean_start_override_before_will_scheduled() {
    let id = MqttString::from_str("WILL_CLEAN_START_BEFORE_WILL_SCHEDULED").unwrap();

    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "The day I died was just like any other idle Thursday.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(100);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .will(will)
        .session_expiry_interval(SessionExpiryInterval::Seconds(100));

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, Some(id.clone())).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
        sleep(Duration::from_secs(4)).await;

        let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);

        let info = assert_ok!(warn_inspect!(
            tx.connect(tcp, &will_connect_options, Some(id.as_borrowed()))
                .await,
            "Client::connect() failed"
        ));
        assert!(!info.session_present);

        sleep(Duration::from_secs(1)).await;

        disconnect(&mut tx, &DisconnectOptions::new()).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        assert_err!(
            timeout(Duration::from_secs(4), async {
                assert_recv!(rx);
            })
            .await,
            "Expected to receive nothing"
        );

        let Publish { topic, .. } = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(3), receive_and_complete(&mut rx)).await
        ));

        assert_eq!(topic, will_topic_name);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "spec self-contradictory, for a discussion see https://github.com/eclipse-mosquitto/mosquitto/issues/3509"]
#[tokio::test]
#[test_log::test]
async fn will_existing_session_taken_over_with_session_expiry() {
    let id = MqttString::from_str("WILL_EXISTING_SESSION_TAKEN_OVER_WITH_SESSION_EXPIRY").unwrap();

    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "In this world, nothing can be certain, except death and taxes.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    );

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .session_expiry_interval(SessionExpiryInterval::Seconds(100))
        .will(will);

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, Some(id.clone())).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher_takeover = async {
        sleep(Duration::from_secs(2)).await;

        let mut takeover_connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        takeover_connect_options.clean_start = false;

        let mut tx_takeover = assert_ok!(
            connected_client(BROKER_ADDRESS, &takeover_connect_options, Some(id.clone())).await
        );

        disconnect(&mut tx_takeover, DEFAULT_DC_OPTIONS).await;
    };

    let publisher = async {
        let e = assert_err!(tx.poll().await);
        assert!(matches!(
            e,
            MqttError::Disconnect {
                reason: ReasonCode::SessionTakenOver,
                reason_string: _,
                server_reference: _,
            }
        ));
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        let Publish { topic, .. } = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(10), receive_and_complete(&mut rx)).await
        ));
        assert_eq!(topic, will_topic_name);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher, publisher_takeover);
}

// fails with mosquitto v2.1.2
#[ignore = "enable this test once mosquitto v2.1.3 is used"]
#[tokio::test]
#[test_log::test]
async fn will_existing_session_taken_over_with_will_delay() {
    let id = MqttString::from_str("WILL_EXISTING_SESSION_TAKEN_OVER_WITH_WILL_DELAY").unwrap();

    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "Life is hard. Then you die. Then they throw dirt in your face. Then the worms eat you. Be grateful if it happens in that order.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(100);

    // Session expiry interval = 0
    let will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .session_expiry_interval(SessionExpiryInterval::EndOnDisconnect)
        .will(will);

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, Some(id.clone())).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher_takeover = async {
        sleep(Duration::from_secs(2)).await;

        let mut takeover_connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        takeover_connect_options.clean_start = false;

        let mut tx_takeover = assert_ok!(
            connected_client(BROKER_ADDRESS, &takeover_connect_options, Some(id.clone())).await
        );

        disconnect(&mut tx_takeover, DEFAULT_DC_OPTIONS).await;
    };

    let publisher = async {
        let e = assert_err!(tx.poll().await);
        assert!(matches!(
            e,
            MqttError::Disconnect {
                reason: ReasonCode::SessionTakenOver,
                reason_string: _,
                server_reference: _,
            }
        ));
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        let Publish { topic, .. } = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(10), receive_and_complete(&mut rx)).await
        ));
        assert_eq!(topic, will_topic_name);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher, publisher_takeover);
}

// fails with mosquitto v2.1.2
#[ignore = "enable this test once mosquitto v2.1.3 is used"]
#[tokio::test]
#[test_log::test]
async fn will_existing_session_taken_over_with_clean_start() {
    let id = MqttString::from_str("WILL_EXISTING_SESSION_TAKEN_OVER_WITH_CLEAN_START").unwrap();

    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "I intend to live forever or die trying.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(100);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .session_expiry_interval(SessionExpiryInterval::Seconds(100))
        .will(will);

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, Some(id.clone())).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher_takeover = async {
        sleep(Duration::from_secs(2)).await;

        let mut tx_takeover = assert_ok!(
            connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, Some(id.clone())).await
        );

        disconnect(&mut tx_takeover, DEFAULT_DC_OPTIONS).await;
    };

    let publisher = async {
        let e = assert_err!(tx.poll().await);
        assert!(matches!(
            e,
            MqttError::Disconnect {
                reason: ReasonCode::SessionTakenOver,
                reason_string: _,
                server_reference: _,
            }
        ));
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        let Publish { topic, .. } = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(10), receive_and_complete(&mut rx)).await
        ));
        assert_eq!(topic, will_topic_name);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher, publisher_takeover);
}

#[tokio::test]
#[test_log::test]
async fn no_will_existing_session_taken_over() {
    let id = MqttString::from_str("WILL_EXISTING_SESSION_TAKEN_OVER").unwrap();

    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "To reach the end of your life and wish you had time for a few other roads — what could be more human?";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(1);

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .session_expiry_interval(SessionExpiryInterval::Seconds(1))
        .will(will);

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, Some(id.clone())).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher_takeover = async {
        sleep(Duration::from_secs(2)).await;

        let mut takeover_connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        takeover_connect_options.clean_start = false;

        let mut tx_takeover = assert_ok!(
            connected_client(BROKER_ADDRESS, &takeover_connect_options, Some(id.clone())).await
        );

        disconnect(&mut tx_takeover, DEFAULT_DC_OPTIONS).await;
    };

    let publisher = async {
        let e = assert_err!(tx.poll().await);
        assert!(matches!(
            e,
            MqttError::Disconnect {
                reason: ReasonCode::SessionTakenOver,
                reason_string: _,
                server_reference: _,
            }
        ));
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

        assert_err!(
            timeout(Duration::from_secs(10), async {
                assert_recv!(rx);
            })
            .await,
            "Expected to receive nothing"
        );

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher, publisher_takeover);
}

#[tokio::test]
#[test_log::test]
async fn session_recovered_before_session_expires() {
    let id = MqttString::from_str("WILL_SESSION_RECOVERED_BEFORE_SESSION_END").unwrap();

    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "Human beings and snakes can both kill you, with one major difference: Human beings will then walk in your funeral, the snakes won't!";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(10);

    let mut will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .will(will)
        .session_expiry_interval(SessionExpiryInterval::Seconds(5));

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, Some(id.clone())).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
        sleep(Duration::from_secs(4)).await;

        will_connect_options.clean_start = false;
        let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);

        let info = assert_ok!(warn_inspect!(
            tx.connect(tcp, &will_connect_options, Some(id.as_borrowed()))
                .await,
            "Client::connect() failed"
        ));
        assert!(info.session_present);

        sleep(Duration::from_secs(2)).await;

        disconnect(&mut tx, &DisconnectOptions::new()).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

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
async fn session_recovered_before_will_delay_interval() {
    let id = MqttString::from_str("WILL_SESSION_RECOVERED_BEFORE_WILL_DELAY_INTERVAL").unwrap();

    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "Death offers you thorns, eternity offers you roses, and life offers you both.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .delay_interval(5);

    let mut will_connect_options = NO_SESSION_CONNECT_OPTIONS
        .clone()
        .will(will)
        .session_expiry_interval(SessionExpiryInterval::Seconds(10));

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, Some(id.clone())).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
        sleep(Duration::from_secs(4)).await;

        will_connect_options.clean_start = false;
        let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);

        let info = assert_ok!(warn_inspect!(
            tx.connect(tcp, &will_connect_options, Some(id.as_borrowed()))
                .await,
            "Client::connect() failed"
        ));
        assert!(info.session_present);

        sleep(Duration::from_secs(2)).await;

        disconnect(&mut tx, &DisconnectOptions::new()).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, will_topic_filter.clone());

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
async fn retain() {
    let (will_topic_name, will_topic_filter) = unique_topic();
    let will_msg = "Don't die before you're dead.";

    let will = WillOptions::new(
        will_topic_name.clone(),
        MqttBinary::from_slice(will_msg.as_bytes()).unwrap(),
    )
    .retain();

    let will_connect_options = NO_SESSION_CONNECT_OPTIONS.clone().will(will);

    let mut tx = assert_ok!(connected_client(BROKER_ADDRESS, &will_connect_options, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut retained_rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            DEFAULT_QOS0_SUB_OPTIONS.retain_as_published(),
            will_topic_filter.clone()
        );

        let Publish { retain, .. } = assert_recv_excl!(rx, will_topic_name);
        assert!(retain);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    let retained_receiver = async {
        sleep(Duration::from_secs(7)).await;

        assert_subscribe!(
            retained_rx,
            DEFAULT_QOS0_SUB_OPTIONS,
            will_topic_filter.clone()
        );

        let Publish { retain, .. } = assert_recv_excl!(retained_rx, will_topic_name);
        assert!(retain);

        disconnect(&mut retained_rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, retained_receiver, publisher);
}
