use std::time::Duration;

use rust_mqtt::{
    client::{
        Client,
        event::{Event, Puback},
        options::{PublicationOptions, TopicReference},
    },
    config::SessionExpiryInterval,
    types::{IdentifiedQoS, MqttString, QoS},
};
use tokio::{
    join,
    time::{sleep, timeout},
};
use tokio_test::assert_err;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, DEFAULT_QOS0_SUB_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::{assert_ok, assert_published, assert_subscribe},
    fmt::warn_inspect,
    utils::{ALLOC, connected_client, disconnect, receive_publish, tcp_connection, unique_topic},
};

#[tokio::test]
#[test_log::test]
async fn outgoing_qos1_retry() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "test message";
    let tx_id = MqttString::from_slice("RETRY_OUTGOING_QOS1_TX_CLIENT").unwrap();

    let mut tx_connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    tx_connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

    let mut tx = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &tx_connect_options,
            Some(tx_id.as_borrowed())
        )
        .await
    );
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions {
            retain: false,
            message_expiry_interval: None,
            topic: TopicReference::Name(topic_name.clone()),
            qos: QoS::AtLeastOnce,
        };

        let pid = assert_ok!(tx.publish(&pub_options, msg.into()).await);
        let session = tx.session().clone();

        drop(tx);

        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::EndOnDisconnect;
        connect_options.clean_start = false;

        let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
        let mut tx: Client<'_, _, _, 1, 1, 1, 1> = Client::with_session(session, ALLOC.get());
        let info = assert_ok!(warn_inspect!(
            tx.connect(tcp, &connect_options, Some(tx_id.as_borrowed()))
                .await,
            "Client::connect() failed"
        ));
        assert!(info.session_present);

        assert_ok!(tx.republish(pid, &pub_options, msg.into()).await);

        assert_ok!(
            timeout(Duration::from_secs(5), async {
                loop {
                    match assert_ok!(tx.poll().await) {
                        Event::PublishAcknowledged(Puback {
                            packet_identifier,
                            reason_code: _,
                        }) if packet_identifier == pid => break,
                        _ => {}
                    }
                }
            })
            .await
        );

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut sub_options = DEFAULT_QOS0_SUB_OPTIONS;
        sub_options.qos = QoS::AtLeastOnce;
        assert_subscribe!(rx, sub_options, topic_filter.clone());
        let p = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(5), receive_publish(&mut rx)).await
        ));
        assert_eq!(p.topic, topic_name.as_borrowed().into());
        assert_eq!(p.message, msg.into());

        let p = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(5), receive_publish(&mut rx)).await
        ));
        assert_eq!(p.topic, topic_name.as_borrowed().into());
        assert_eq!(p.message, msg.into());

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn outgoing_qos2_retry_publish() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "test message";
    let tx_id = MqttString::from_slice("RETRY_OUTGOING_QOS2_PUBLISH_TX_CLIENT").unwrap();

    let mut tx_connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    tx_connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

    let mut tx = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &tx_connect_options,
            Some(tx_id.as_borrowed())
        )
        .await
    );
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions {
            retain: false,
            message_expiry_interval: None,
            topic: TopicReference::Name(topic_name.clone()),
            qos: QoS::ExactlyOnce,
        };

        let pid = assert_ok!(tx.publish(&pub_options, msg.into()).await);
        let session = tx.session().clone();

        drop(tx);

        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::EndOnDisconnect;
        connect_options.clean_start = false;

        let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
        let mut tx: Client<'_, _, _, 1, 1, 1, 1> = Client::with_session(session, ALLOC.get());
        let info = assert_ok!(warn_inspect!(
            tx.connect(tcp, &connect_options, Some(tx_id.as_borrowed()))
                .await,
            "Client::connect() failed"
        ));
        assert!(info.session_present);

        assert_ok!(tx.republish(pid, &pub_options, msg.into()).await);

        assert_ok!(
            timeout(Duration::from_secs(5), async {
                loop {
                    match assert_ok!(tx.poll().await) {
                        Event::PublishComplete(Puback {
                            packet_identifier,
                            reason_code: _,
                        }) if packet_identifier == pid => break,
                        _ => {}
                    }
                }
            })
            .await
        );

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut sub_options = DEFAULT_QOS0_SUB_OPTIONS;
        sub_options.qos = QoS::ExactlyOnce;
        assert_subscribe!(rx, sub_options, topic_filter.clone());
        let p = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(5), receive_publish(&mut rx)).await
        ));
        assert_eq!(p.topic, topic_name.as_borrowed().into());
        assert_eq!(p.message, msg.into());

        assert_err!(timeout(Duration::from_secs(5), receive_publish(&mut rx)).await);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn outgoing_qos2_retry_pubrel() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "test message";
    let tx_id = MqttString::from_slice("RETRY_OUTGOING_QOS2_PUBREL_TX_CLIENT").unwrap();

    let mut tx_connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    tx_connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

    let mut tx = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &tx_connect_options,
            Some(tx_id.as_borrowed())
        )
        .await
    );
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions {
            retain: false,
            message_expiry_interval: None,
            topic: TopicReference::Name(topic_name.clone()),
            qos: QoS::ExactlyOnce,
        };

        let pid = assert_ok!(tx.publish(&pub_options, msg.into()).await);

        assert_ok!(
            timeout(Duration::from_secs(5), async {
                loop {
                    match assert_ok!(tx.poll().await) {
                        Event::PublishReceived(Puback {
                            packet_identifier,
                            reason_code: _,
                        }) if packet_identifier == pid => break,
                        _ => {}
                    }
                }
            })
            .await
        );

        let session = tx.session().clone();

        drop(tx);

        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::EndOnDisconnect;
        connect_options.clean_start = false;

        let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
        let mut tx: Client<'_, _, _, 1, 1, 1, 1> = Client::with_session(session, ALLOC.get());
        let info = assert_ok!(warn_inspect!(
            tx.connect(tcp, &connect_options, Some(tx_id.as_borrowed()))
                .await,
            "Client::connect() failed"
        ));
        assert!(info.session_present);

        assert_ok!(tx.rerelease().await);

        assert_ok!(
            timeout(Duration::from_secs(5), async {
                loop {
                    match assert_ok!(tx.poll().await) {
                        Event::PublishComplete(Puback {
                            packet_identifier,
                            reason_code: _,
                        }) if packet_identifier == pid => break,
                        _ => {}
                    }
                }
            })
            .await
        );

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut sub_options = DEFAULT_QOS0_SUB_OPTIONS;
        sub_options.qos = QoS::ExactlyOnce;
        assert_subscribe!(rx, sub_options, topic_filter.clone());
        let p = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(5), receive_publish(&mut rx)).await
        ));
        assert_eq!(p.topic, topic_name.as_borrowed().into());
        assert_eq!(p.message, msg.into());

        assert_err!(timeout(Duration::from_secs(5), receive_publish(&mut rx)).await);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn incoming_qos2_retry_pubcomp() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "test message";
    let rx_id = MqttString::from_slice("RETRY_INCOMING_QOS2_RX_CLIENT").unwrap();

    let mut rx_connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    rx_connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            &rx_connect_options,
            Some(rx_id.as_borrowed())
        )
        .await
    );

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions {
            retain: false,
            message_expiry_interval: None,
            topic: TopicReference::Name(topic_name.clone()),
            qos: QoS::ExactlyOnce,
        };

        assert_published!(tx, &pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut sub_options = DEFAULT_QOS0_SUB_OPTIONS;
        sub_options.qos = QoS::ExactlyOnce;
        assert_subscribe!(rx, sub_options, topic_filter.clone());

        let publish = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(5), receive_publish(&mut rx)).await
        ));
        assert_eq!(
            <IdentifiedQoS as Into<QoS>>::into(publish.identified_qos),
            QoS::ExactlyOnce
        );
        assert!(publish.identified_qos.packet_identifier().is_some());
        let pid = publish.identified_qos.packet_identifier().unwrap();
        assert_eq!(publish.topic, topic_name.as_borrowed().into());
        assert_eq!(publish.message, msg.into());

        let session = rx.session().clone();

        drop(rx);

        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::EndOnDisconnect;
        connect_options.clean_start = false;

        let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
        let mut rx: Client<'_, _, _, 1, 1, 1, 1> = Client::with_session(session, ALLOC.get());
        let info = assert_ok!(warn_inspect!(
            rx.connect(tcp, &connect_options, Some(rx_id.as_borrowed()))
                .await,
            "Client::connect() failed"
        ));
        assert!(info.session_present);

        assert_ok!(
            timeout(Duration::from_secs(5), async {
                loop {
                    match assert_ok!(rx.poll().await) {
                        Event::PublishReleased(Puback {
                            packet_identifier,
                            reason_code,
                        }) if packet_identifier == pid => {
                            break;
                        }
                        _ => {}
                    }
                }
            })
            .await
        );
        assert_err!(timeout(Duration::from_secs(5), receive_publish(&mut rx)).await);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}
