use std::time::Duration;

use rust_mqtt::{
    client::{
        Client,
        event::{Event, Puback, Publish, Suback},
        options::{PublicationOptions, TopicReference},
    },
    config::SessionExpiryInterval,
    session::{CPublishFlightState, InFlightPublish},
    types::{IdentifiedQoS, MqttString, QoS, TopicFilter, TopicName},
};
use tokio::{
    join,
    sync::{mpsc, oneshot},
    time::{sleep, timeout},
};
use tokio_test::assert_err;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, DEFAULT_QOS0_SUB_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::{assert_ok, assert_published, assert_subscribe},
    failing::ByteLimit,
    fmt::warn_inspect,
    utils::{
        ALLOC, connected_client, connected_failing_client, disconnect, receive_and_complete,
        receive_publish, tcp_connection, unique_topic,
    },
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

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).at_least_once();

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
        assert_eq!(p.topic, topic_name.as_borrowed());
        assert_eq!(p.message, msg.into());

        let p = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(5), receive_publish(&mut rx)).await
        ));
        assert_eq!(p.topic, topic_name.as_borrowed());
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

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

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
        assert_eq!(p.topic, topic_name.as_borrowed());
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

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

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
        assert_eq!(p.topic, topic_name.as_borrowed());
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

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

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
        assert_eq!(publish.topic, topic_name.as_borrowed());
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

#[tokio::test]
#[test_log::test]
async fn outgoing_qos1_write_fail_retry() {
    let tx_id = MqttString::from_slice("RETRY_OUTGOING_QOS1_WRITE_FAIL_CLIENT").unwrap();

    let (rx_subscribed, subscribed) = oneshot::channel();
    let (messages, rx_messages) = mpsc::channel(1);
    let (rx_received, mut received) = mpsc::channel(1);

    let (topic_name, topic_filter) = unique_topic();

    let rx = receiver_task(topic_filter, rx_subscribed, rx_messages, rx_received);

    let tx = async move {
        assert_ok!(subscribed.await);

        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

        let mut reconnect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        reconnect_options.clean_start = false;

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).at_least_once();

        let mut i = 0;
        let mut failed = true;

        'outer: while failed {
            let message = i.to_string().into_bytes();

            i += 1;
            failed = false;

            'republish: {
                let session = 'try_publish: {
                    let Ok(mut tx) = connected_failing_client(
                        BROKER_ADDRESS,
                        &connect_options,
                        Some(tx_id.as_borrowed()),
                        ByteLimit::Unlimited,
                        ByteLimit::FailAfter(i),
                    )
                    .await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    let Ok(pid) = tx.publish(&pub_options, message.as_slice().into()).await else {
                        failed = true;
                        break 'try_publish tx.session().clone();
                    };

                    // Cannot fail on receiving messages
                    match assert_ok!(tx.poll().await) {
                        Event::PublishAcknowledged(Puback {
                            packet_identifier,
                            reason_code: _,
                        }) if pid == packet_identifier => {}
                        _ => panic!("Should only receive a PUBACK"),
                    }

                    let _ = tx.disconnect(DEFAULT_DC_OPTIONS).await;
                    break 'republish;
                };

                // Complete publish using infallible connection

                let pid = session.pending_client_publishes.first().copied();

                let mut tx: Client<'_, _, _, 1, 1, 1, 1> =
                    Client::with_session(session, ALLOC.get());
                let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
                assert_ok!(
                    tx.connect(tcp, &reconnect_options, Some(tx_id.as_borrowed()))
                        .await
                );

                let pid = match pid {
                    Some(InFlightPublish {
                        packet_identifier,
                        state: _,
                    }) => {
                        assert_ok!(
                            tx.republish(
                                packet_identifier,
                                &pub_options,
                                message.as_slice().into()
                            )
                            .await
                        );
                        packet_identifier
                    }
                    None => assert_ok!(tx.publish(&pub_options, message.as_slice().into()).await),
                };

                match assert_ok!(tx.poll().await) {
                    Event::PublishAcknowledged(Puback {
                        packet_identifier,
                        reason_code: _,
                    }) if pid == packet_identifier => {}
                    _ => panic!("Should only receive a PUBACK"),
                }
            }

            messages.send(message).await.unwrap();
            assert!(assert_ok!(timeout(Duration::from_secs(5), received.recv()).await).is_some());
        }
    };

    join!(rx, tx);
}

#[tokio::test]
#[test_log::test]
async fn outgoing_qos1_read_fail_retry() {
    let tx_id = MqttString::from_slice("RETRY_OUTGOING_QOS1_READ_FAIL_CLIENT").unwrap();

    let (rx_subscribed, subscribed) = oneshot::channel();
    let (messages, rx_messages) = mpsc::channel(1);
    let (rx_received, mut received) = mpsc::channel(1);

    let (topic_name, topic_filter) = unique_topic();

    let rx = receiver_task(topic_filter, rx_subscribed, rx_messages, rx_received);

    let tx = async move {
        assert_ok!(subscribed.await);

        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

        let mut reconnect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        reconnect_options.clean_start = false;

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).at_least_once();

        let mut i = 0;
        let mut failed = true;

        'outer: while failed {
            let message = i.to_string().into_bytes();

            i += 1;
            failed = false;

            'republish: {
                let session = 'try_publish: {
                    let Ok(mut tx) = connected_failing_client(
                        BROKER_ADDRESS,
                        &connect_options,
                        Some(tx_id.as_borrowed()),
                        ByteLimit::FailAfter(i),
                        ByteLimit::Unlimited,
                    )
                    .await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    // Cannot fail on sending messages
                    let pid = assert_ok!(tx.publish(&pub_options, message.as_slice().into()).await);

                    match tx.poll().await {
                        Ok(Event::PublishAcknowledged(Puback {
                            packet_identifier,
                            reason_code: _,
                        })) if pid == packet_identifier => {}
                        Ok(_) => panic!("Should only receive a PUBACK"),
                        Err(_) => {
                            failed = true;
                            break 'try_publish tx.session().clone();
                        }
                    }

                    let _ = tx.disconnect(DEFAULT_DC_OPTIONS).await;
                    break 'republish;
                };

                // Complete publish

                let pid = session
                    .pending_client_publishes
                    .first()
                    .unwrap()
                    .packet_identifier;

                let mut tx: Client<'_, _, _, 1, 1, 1, 1> =
                    Client::with_session(session, ALLOC.get());
                let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
                assert_ok!(
                    tx.connect(tcp, &reconnect_options, Some(tx_id.as_borrowed()))
                        .await
                );

                assert_ok!(
                    tx.republish(pid, &pub_options, message.as_slice().into())
                        .await
                );

                match assert_ok!(tx.poll().await) {
                    Event::PublishAcknowledged(Puback {
                        packet_identifier,
                        reason_code: _,
                    }) if pid == packet_identifier => {}
                    _ => panic!("Should only receive a PUBACK"),
                }
            }

            messages.send(message).await.unwrap();
            assert!(assert_ok!(timeout(Duration::from_secs(5), received.recv()).await).is_some());
        }
    };

    join!(rx, tx);
}

#[tokio::test]
#[test_log::test]
async fn outgoing_qos2_write_fail_retry() {
    let tx_id = MqttString::from_slice("RETRY_OUTGOING_QOS2_WRITE_FAIL_CLIENT").unwrap();

    let (rx_subscribed, subscribed) = oneshot::channel();
    let (messages, rx_messages) = mpsc::channel(1);
    let (rx_received, mut received) = mpsc::channel(1);

    let (topic_name, topic_filter) = unique_topic();

    let rx = receiver_task(topic_filter, rx_subscribed, rx_messages, rx_received);

    let tx = async move {
        assert_ok!(subscribed.await);

        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

        let mut reconnect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        reconnect_options.clean_start = false;

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

        let mut i = 0;
        let mut failed = true;

        'outer: while failed {
            let message = i.to_string().into_bytes();

            i += 1;
            failed = false;

            'republish: {
                let session = 'try_publish: {
                    let Ok(mut tx) = connected_failing_client(
                        BROKER_ADDRESS,
                        &connect_options,
                        Some(tx_id.as_borrowed()),
                        ByteLimit::Unlimited,
                        ByteLimit::FailAfter(i),
                    )
                    .await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    let Ok(pid) = tx.publish(&pub_options, message.as_slice().into()).await else {
                        failed = true;
                        break 'try_publish tx.session().clone();
                    };

                    // Can fail because we have to responde with PUBREL
                    match tx.poll().await {
                        Ok(Event::PublishReceived(Puback {
                            packet_identifier,
                            reason_code: _,
                        })) if pid == packet_identifier => {}
                        Ok(_) => panic!("Should only receive a PUBREC"),
                        Err(_) => {
                            failed = true;
                            break 'try_publish tx.session().clone();
                        }
                    }

                    // Cannot fail because PUBCOMP is unanswered
                    match assert_ok!(tx.poll().await) {
                        Event::PublishComplete(Puback {
                            packet_identifier,
                            reason_code: _,
                        }) if pid == packet_identifier => {}
                        _ => panic!("Should only receive a PUBCOMP"),
                    }

                    let _ = tx.disconnect(DEFAULT_DC_OPTIONS).await;
                    break 'republish;
                };

                // Complete publish using infallible connection

                let pid = session.pending_client_publishes.first().copied();

                let mut tx: Client<'_, _, _, 1, 1, 1, 1> =
                    Client::with_session(session, ALLOC.get());
                let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
                assert_ok!(
                    tx.connect(tcp, &reconnect_options, Some(tx_id.as_borrowed()))
                        .await
                );

                let (pid, wait_for_pubrec) = match pid {
                    Some(InFlightPublish {
                        packet_identifier,
                        state: CPublishFlightState::AwaitingPubrec,
                    }) => {
                        assert_ok!(
                            tx.republish(
                                packet_identifier,
                                &pub_options,
                                message.as_slice().into()
                            )
                            .await
                        );
                        (packet_identifier, true)
                    }
                    Some(InFlightPublish {
                        packet_identifier,
                        state: CPublishFlightState::AwaitingPubcomp,
                    }) => {
                        assert_ok!(tx.rerelease().await);
                        (packet_identifier, false)
                    }
                    Some(_) => unreachable!("Should only have QoS 2 states"),
                    None => (
                        assert_ok!(tx.publish(&pub_options, message.as_slice().into()).await),
                        true,
                    ),
                };

                if wait_for_pubrec {
                    match assert_ok!(tx.poll().await) {
                        Event::PublishReceived(Puback {
                            packet_identifier,
                            reason_code: _,
                        }) if pid == packet_identifier => {}
                        _ => panic!("Should only receive a PUBREC"),
                    }
                }

                match assert_ok!(tx.poll().await) {
                    Event::PublishComplete(Puback {
                        packet_identifier,
                        reason_code: _,
                    }) if pid == packet_identifier => {}
                    _ => panic!("Should only receive a PUBCOMP"),
                }
            }

            messages.send(message).await.unwrap();
            assert!(assert_ok!(timeout(Duration::from_secs(5), received.recv()).await).is_some());
        }
    };

    join!(rx, tx);
}

#[tokio::test]
#[test_log::test]
async fn outgoing_qos2_read_fail_retry() {
    let tx_id = MqttString::from_slice("RETRY_OUTGOING_QOS2_READ_FAIL_CLIENT").unwrap();

    let (rx_subscribed, subscribed) = oneshot::channel();
    let (messages, rx_messages) = mpsc::channel(1);
    let (rx_received, mut received) = mpsc::channel(1);

    let (topic_name, topic_filter) = unique_topic();

    let rx = receiver_task(topic_filter, rx_subscribed, rx_messages, rx_received);

    let tx = async move {
        assert_ok!(subscribed.await);

        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

        let mut reconnect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        reconnect_options.clean_start = false;

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();

        let mut i = 0;
        let mut failed = true;

        'outer: while failed {
            let message = i.to_string().into_bytes();

            i += 1;
            failed = false;

            'republish: {
                let session = 'try_publish: {
                    let Ok(mut tx) = connected_failing_client(
                        BROKER_ADDRESS,
                        &connect_options,
                        Some(tx_id.as_borrowed()),
                        ByteLimit::FailAfter(i),
                        ByteLimit::Unlimited,
                    )
                    .await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    let pid = assert_ok!(tx.publish(&pub_options, message.as_slice().into()).await);

                    match tx.poll().await {
                        Ok(Event::PublishReceived(Puback {
                            packet_identifier,
                            reason_code: _,
                        })) if pid == packet_identifier => {}
                        Ok(_) => panic!("Should only receive a PUBREC"),
                        Err(_) => {
                            failed = true;
                            break 'try_publish tx.session().clone();
                        }
                    }

                    match tx.poll().await {
                        Ok(Event::PublishComplete(Puback {
                            packet_identifier,
                            reason_code: _,
                        })) if pid == packet_identifier => {}
                        Ok(_) => panic!("Should only receive a PUBCOMP"),
                        Err(_) => {
                            failed = true;
                            break 'try_publish tx.session().clone();
                        }
                    }

                    let _ = tx.disconnect(DEFAULT_DC_OPTIONS).await;
                    break 'republish;
                };

                // Complete publish using infallible connection

                let pid = session.pending_client_publishes.first().copied();

                let mut tx: Client<'_, _, _, 1, 1, 1, 1> =
                    Client::with_session(session, ALLOC.get());
                let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
                assert_ok!(
                    tx.connect(tcp, &reconnect_options, Some(tx_id.as_borrowed()))
                        .await
                );

                let (pid, wait_for_pubrec) = match pid {
                    Some(InFlightPublish {
                        packet_identifier,
                        state: CPublishFlightState::AwaitingPubrec,
                    }) => {
                        assert_ok!(
                            tx.republish(
                                packet_identifier,
                                &pub_options,
                                message.as_slice().into()
                            )
                            .await
                        );
                        (packet_identifier, true)
                    }
                    Some(InFlightPublish {
                        packet_identifier,
                        state: CPublishFlightState::AwaitingPubcomp,
                    }) => {
                        assert_ok!(tx.rerelease().await);
                        (packet_identifier, false)
                    }
                    Some(_) => unreachable!("Should only have QoS 2 states"),
                    None => (
                        assert_ok!(tx.publish(&pub_options, message.as_slice().into()).await),
                        true,
                    ),
                };

                if wait_for_pubrec {
                    match assert_ok!(tx.poll().await) {
                        Event::PublishReceived(Puback {
                            packet_identifier,
                            reason_code: _,
                        }) if pid == packet_identifier => {}
                        _ => panic!("Should only receive a PUBREC"),
                    }
                }

                match assert_ok!(tx.poll().await) {
                    Event::PublishComplete(Puback {
                        packet_identifier,
                        reason_code: _,
                    }) if pid == packet_identifier => {}
                    _ => panic!("Should only receive a PUBCOMP"),
                }
            }

            messages.send(message).await.unwrap();
            assert!(assert_ok!(timeout(Duration::from_secs(5), received.recv()).await).is_some());
        }
    };

    join!(rx, tx);
}

async fn receiver_task(
    topic: TopicFilter<'static>,
    confirm_subscribed: oneshot::Sender<()>,
    mut messages: mpsc::Receiver<Vec<u8>>,
    confirm_received: mpsc::Sender<()>,
) {
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, topic);

    confirm_subscribed.send(()).unwrap();

    while let Some(content) = messages.recv().await {
        loop {
            let p = assert_ok!(receive_and_complete(&mut rx).await);

            if *p.message == content {
                confirm_received.send(()).await.unwrap();
                break;
            }
        }
    }

    disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn incoming_qos1_write_fail_retry() {
    let tx_id = MqttString::from_slice("RETRY_INCOMING_QOS1_WRITE_FAIL_CLIENT").unwrap();

    let (messages, tx_messages) = mpsc::channel(1);
    let (received, tx_received) = mpsc::channel(1);

    let (topic_name, topic_filter) = unique_topic();

    let tx = publisher_task(topic_name, QoS::AtLeastOnce, tx_messages, tx_received);

    let rx = async move {
        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

        let mut reconnect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        reconnect_options.clean_start = false;

        let mut sub_options = DEFAULT_QOS0_SUB_OPTIONS;
        sub_options.qos = QoS::AtLeastOnce;

        let mut i = 0;
        let mut failed = true;

        'outer: while failed {
            let message = i.to_string().into_bytes();

            i += 1;
            failed = false;

            'rereceive: {
                let msg = message.clone();

                let session = 'try_receive: {
                    let Ok(mut rx) = connected_failing_client(
                        BROKER_ADDRESS,
                        &connect_options,
                        Some(tx_id.as_borrowed()),
                        ByteLimit::Unlimited,
                        ByteLimit::FailAfter(i),
                    )
                    .await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    let Ok(pid) = rx.subscribe(topic_filter.as_borrowed(), sub_options).await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    // Cannot fail because SUBACK is unanswered
                    match assert_ok!(rx.poll().await) {
                        Event::Suback(Suback {
                            packet_identifier,
                            reason_code: _,
                        }) if pid == packet_identifier => {}
                        _ => panic!("Should only receive a SUBACK"),
                    }

                    messages.send(message).await.unwrap();

                    match rx.poll().await {
                        Ok(Event::Publish(Publish {
                            identified_qos: IdentifiedQoS::AtLeastOnce(packet_identifier),
                            dup: false,
                            message,
                            ..
                        })) if &*message == msg.as_slice() => {}
                        Ok(Event::Publish(_)) => panic!("Received non-matching PUBLISH"),
                        Ok(_) => panic!("Should only receive a PUBLISH"),
                        Err(_) => {
                            failed = true;
                            break 'try_receive rx.session().clone();
                        }
                    };

                    break 'rereceive;
                };

                // Complete publish using infallible connection

                let mut rx: Client<'_, _, _, 1, 1, 1, 1> =
                    Client::with_session(session, ALLOC.get());
                let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
                assert_ok!(
                    rx.connect(tcp, &reconnect_options, Some(tx_id.as_borrowed()))
                        .await
                );

                match assert_ok!(rx.poll().await) {
                    Event::Publish(Publish {
                        identified_qos: IdentifiedQoS::AtLeastOnce(packet_identifier),
                        dup: true,
                        message,
                        ..
                    }) if &*message == msg.as_slice() => {}
                    Event::Publish(p) => panic!("Received non-matching PUBLISH: {:?}", p),
                    _ => panic!("Should only receive a PUBLISH"),
                };
            }

            received.send(()).await.unwrap();
        }
    };

    join!(rx, tx);
}

#[tokio::test]
#[test_log::test]
async fn incoming_qos1_read_fail_retry() {
    let tx_id = MqttString::from_slice("RETRY_INCOMING_QOS1_READ_FAIL_CLIENT").unwrap();

    let (messages, tx_messages) = mpsc::channel(1);
    let (received, tx_received) = mpsc::channel(1);

    let (topic_name, topic_filter) = unique_topic();

    let tx = publisher_task(topic_name, QoS::AtLeastOnce, tx_messages, tx_received);

    let rx = async move {
        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

        let mut reconnect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        reconnect_options.clean_start = false;

        let mut sub_options = DEFAULT_QOS0_SUB_OPTIONS;
        sub_options.qos = QoS::AtLeastOnce;

        let mut i = 0;
        let mut failed = true;

        'outer: while failed {
            let message = i.to_string().into_bytes();

            i += 1;
            failed = false;

            'rereceive: {
                let msg = message.clone();

                let session = 'try_receive: {
                    let Ok(mut rx) = connected_failing_client(
                        BROKER_ADDRESS,
                        &connect_options,
                        Some(tx_id.as_borrowed()),
                        ByteLimit::FailAfter(i),
                        ByteLimit::Unlimited,
                    )
                    .await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    // Cannot fail because subscribing only sends
                    let pid =
                        assert_ok!(rx.subscribe(topic_filter.as_borrowed(), sub_options).await);

                    match rx.poll().await {
                        Ok(Event::Suback(Suback {
                            packet_identifier,
                            reason_code: _,
                        })) if pid == packet_identifier => {}
                        Ok(_) => panic!("Should only receive a SUBACK"),
                        Err(_) => {
                            failed = true;
                            continue 'outer;
                        }
                    }

                    messages.send(message).await.unwrap();

                    match rx.poll().await {
                        Ok(Event::Publish(Publish {
                            identified_qos: IdentifiedQoS::AtLeastOnce(packet_identifier),
                            dup: false,
                            message,
                            ..
                        })) if &*message == msg.as_slice() => {}
                        Ok(Event::Publish(_)) => panic!("Received non-matching PUBLISH"),
                        Ok(_) => panic!("Should only receive a PUBLISH"),
                        Err(_) => {
                            failed = true;
                            break 'try_receive rx.session().clone();
                        }
                    };

                    break 'rereceive;
                };

                // Complete publish using infallible connection

                let mut rx: Client<'_, _, _, 1, 1, 1, 1> =
                    Client::with_session(session, ALLOC.get());
                let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
                assert_ok!(
                    rx.connect(tcp, &reconnect_options, Some(tx_id.as_borrowed()))
                        .await
                );

                match assert_ok!(rx.poll().await) {
                    Event::Publish(Publish {
                        identified_qos: IdentifiedQoS::AtLeastOnce(packet_identifier),
                        message,
                        ..
                    }) if &*message == msg.as_slice() => {}
                    Event::Publish(p) => panic!("Received non-matching PUBLISH: {:?}", p),
                    _ => panic!("Should only receive a PUBLISH"),
                };
            }

            received.send(()).await.unwrap();
        }
    };

    join!(rx, tx);
}

#[tokio::test]
#[test_log::test]
async fn incoming_qos2_write_fail_retry() {
    let tx_id = MqttString::from_slice("RETRY_INCOMING_QOS2_WRITE_FAIL_CLIENT").unwrap();

    let (messages, tx_messages) = mpsc::channel(1);
    let (received, tx_received) = mpsc::channel(1);

    let (topic_name, topic_filter) = unique_topic();

    let tx = publisher_task(topic_name, QoS::ExactlyOnce, tx_messages, tx_received);

    let rx = async move {
        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

        let mut reconnect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        reconnect_options.clean_start = false;

        let mut sub_options = DEFAULT_QOS0_SUB_OPTIONS;
        sub_options.qos = QoS::ExactlyOnce;

        let mut i = 0;
        let mut failed = true;

        'outer: while failed {
            let message = i.to_string().into_bytes();

            i += 1;
            failed = false;

            'rereceive: {
                let msg = message.clone();

                let session = 'try_receive: {
                    let Ok(mut rx) = connected_failing_client(
                        BROKER_ADDRESS,
                        &connect_options,
                        Some(tx_id.as_borrowed()),
                        ByteLimit::Unlimited,
                        ByteLimit::FailAfter(i),
                    )
                    .await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    let Ok(pid) = rx.subscribe(topic_filter.as_borrowed(), sub_options).await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    // Cannot fail because SUBACK is unanswered
                    match assert_ok!(rx.poll().await) {
                        Event::Suback(Suback {
                            packet_identifier,
                            reason_code: _,
                        }) if pid == packet_identifier => {}
                        _ => panic!("Should only receive a SUBACK"),
                    }

                    messages.send(message).await.unwrap();

                    let pid = match rx.poll().await {
                        Ok(Event::Publish(Publish {
                            identified_qos: IdentifiedQoS::ExactlyOnce(packet_identifier),
                            dup: false,
                            message,
                            ..
                        })) if &*message == msg.as_slice() => pid,
                        Ok(Event::Publish(_)) => panic!("Received non-matching PUBLISH"),
                        Ok(_) => panic!("Should only receive a PUBLISH"),
                        Err(_) => {
                            failed = true;
                            break 'try_receive rx.session().clone();
                        }
                    };

                    match rx.poll().await {
                        Ok(Event::PublishReleased(Puback {
                            packet_identifier,
                            reason_code: _,
                        })) if packet_identifier == pid => {}
                        Ok(Event::PublishReleased(_)) => panic!("Received non-matching PUBREL"),
                        Ok(_) => panic!("Should only receive a PUBREL"),
                        Err(_) => {
                            failed = true;
                            break 'try_receive rx.session().clone();
                        }
                    }

                    break 'rereceive;
                };

                // Complete publish using infallible connection
                // This is only Some(pid) when we haven't received PUBREL yet.
                let pid = session
                    .pending_server_publishes
                    .first()
                    .map(|c| c.packet_identifier);

                let mut rx: Client<'_, _, _, 1, 1, 1, 1> =
                    Client::with_session(session, ALLOC.get());
                let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
                assert_ok!(
                    rx.connect(tcp, &reconnect_options, Some(tx_id.as_borrowed()))
                        .await
                );

                loop {
                    match pid {
                        Some(pid) => match assert_ok!(rx.poll().await) {
                            Event::Duplicate => {}
                            Event::PublishReleased(Puback {
                                packet_identifier,
                                reason_code,
                            }) if packet_identifier == pid => break,
                            Event::PublishReleased(_) => panic!("Received non-matching PUBREL"),
                            e => panic!("Should only receive PUBLISH or PUBREL: {:?}", e),
                        },
                        None => match assert_ok!(rx.poll().await) {
                            Event::Ignored => break,
                            e => panic!("Should only receive unknown PUBCOMP: {:?}", e),
                        },
                    }
                }
            }

            received.send(()).await.unwrap();
        }
    };

    join!(rx, tx);
}

#[tokio::test]
#[test_log::test]
async fn incoming_qos2_read_fail_retry() {
    let tx_id = MqttString::from_slice("RETRY_INCOMING_QOS2_READ_FAIL_CLIENT").unwrap();

    let (messages, tx_messages) = mpsc::channel(1);
    let (received, tx_received) = mpsc::channel(1);

    let (topic_name, topic_filter) = unique_topic();

    let tx = publisher_task(topic_name, QoS::ExactlyOnce, tx_messages, tx_received);

    let rx = async move {
        let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(60);

        let mut reconnect_options = NO_SESSION_CONNECT_OPTIONS.clone();
        reconnect_options.clean_start = false;

        let mut sub_options = DEFAULT_QOS0_SUB_OPTIONS;
        sub_options.qos = QoS::ExactlyOnce;

        let mut i = 0;
        let mut failed = true;

        'outer: while failed {
            let message = i.to_string().into_bytes();

            i += 1;
            failed = false;

            'rereceive: {
                let msg = message.clone();

                let session = 'try_receive: {
                    let Ok(mut rx) = connected_failing_client(
                        BROKER_ADDRESS,
                        &connect_options,
                        Some(tx_id.as_borrowed()),
                        ByteLimit::FailAfter(i),
                        ByteLimit::Unlimited,
                    )
                    .await
                    else {
                        failed = true;
                        continue 'outer;
                    };

                    // Cannot fail because subscribing only sends
                    let pid =
                        assert_ok!(rx.subscribe(topic_filter.as_borrowed(), sub_options).await);

                    match rx.poll().await {
                        Ok(Event::Suback(Suback {
                            packet_identifier,
                            reason_code: _,
                        })) if pid == packet_identifier => {}
                        Ok(_) => panic!("Should only receive a SUBACK"),
                        Err(_) => {
                            failed = true;
                            continue 'outer;
                        }
                    }

                    messages.send(message).await.unwrap();

                    let pid = match rx.poll().await {
                        Ok(Event::Publish(Publish {
                            identified_qos: IdentifiedQoS::ExactlyOnce(packet_identifier),
                            dup: false,
                            message,
                            ..
                        })) if &*message == msg.as_slice() => pid,
                        Ok(Event::Publish(_)) => panic!("Received non-matching PUBLISH"),
                        Ok(_) => panic!("Should only receive a PUBLISH"),
                        Err(_) => {
                            failed = true;
                            break 'try_receive rx.session().clone();
                        }
                    };

                    match rx.poll().await {
                        Ok(Event::PublishReleased(Puback {
                            packet_identifier,
                            reason_code: _,
                        })) if packet_identifier == pid => {}
                        Ok(Event::PublishReleased(_)) => panic!("Received non-matching PUBREL"),
                        Ok(_) => panic!("Should only receive a PUBREL"),
                        Err(_) => {
                            failed = true;
                            break 'try_receive rx.session().clone();
                        }
                    }

                    break 'rereceive;
                };

                // Complete publish using infallible connection
                // This is only Some(pid) when we haven't received PUBREL yet.
                let pid = session
                    .pending_server_publishes
                    .first()
                    .map(|c| c.packet_identifier);

                let mut rx: Client<'_, _, _, 1, 1, 1, 1> =
                    Client::with_session(session, ALLOC.get());
                let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
                assert_ok!(
                    rx.connect(tcp, &reconnect_options, Some(tx_id.as_borrowed()))
                        .await
                );

                'complete: {
                    let pid = match assert_ok!(rx.poll().await) {
                        // We received the missing PUBCOMP
                        Event::Ignored => break 'complete,
                        Event::PublishReleased(Puback {
                            packet_identifier,
                            reason_code,
                        }) if packet_identifier == pid.unwrap() => break 'complete,
                        Event::PublishReleased(_) => panic!("Received non-matching PUBREL"),
                        Event::Publish(Publish {
                            identified_qos: IdentifiedQoS::ExactlyOnce(packet_identifier),
                            message,
                            ..
                        }) if &*message == msg.as_slice() => packet_identifier,
                        Event::Publish(_) => panic!("Received non-matching PUBLISH"),
                        e => panic!("Should only receive PUBLISH or PUBREL: {:?}", e),
                    };

                    match assert_ok!(rx.poll().await) {
                        Event::PublishReleased(Puback {
                            packet_identifier,
                            reason_code,
                        }) if packet_identifier == pid => break 'complete,
                        Event::PublishReleased(_) => panic!("Received non-matching PUBREL"),
                        e => panic!("Should only receive PUBREL: {:?}", e),
                    }
                }
            }

            received.send(()).await.unwrap();
        }
    };

    join!(rx, tx);
}

async fn publisher_task(
    topic: TopicName<'static>,
    qos: QoS,
    mut messages: mpsc::Receiver<Vec<u8>>,
    mut confirm_received: mpsc::Receiver<()>,
) {
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publish_options = PublicationOptions::new(TopicReference::Name(topic)).qos(qos);

    while let Some(content) = messages.recv().await {
        assert_published!(tx, publish_options, content.as_slice().into());

        assert!(
            assert_ok!(timeout(Duration::from_secs(5), confirm_received.recv()).await).is_some()
        );
    }

    disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
}
