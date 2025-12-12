use std::time::Duration;

use rust_mqtt::{
    client::options::{PublicationOptions, RetainHandling},
    types::{MqttString, QoS},
};
use tokio::{
    net::TcpStream,
    time::{sleep, timeout},
};
use tokio_test::assert_err;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, DEFAULT_QOS0_SUB_OPTIONS, NO_SESSION_CONNECT_OPTIONS, Tcp,
    assert::{assert_ok, assert_published, assert_recv, assert_subscribe},
    utils::{connected_client, disconnect, unique_topic},
};

#[tokio::test]
#[test_log::test]
async fn publish_no_local() {
    let topic = unique_topic();
    let msg = "Mosquitto bit me.";

    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let mut options = DEFAULT_QOS0_SUB_OPTIONS;
    options.qos = QoS::ExactlyOnce;
    options.no_local = true;
    assert_subscribe!(c, options, topic.clone());

    let pub_options = PublicationOptions {
        retain: false,
        topic: topic.clone(),
        qos: QoS::ExactlyOnce,
    };

    assert_published!(c, pub_options, msg.into());

    assert_err!(
        timeout(Duration::from_secs(10), async {
            assert_recv!(c);
        })
        .await,
        "Expected to receive nothing"
    );

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn subscribe_retain_handling_default() {
    let topic = unique_topic();
    let msg = "Retained message for AlwaysSend.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    // Receiver client with specific ID to test session behavior
    let rx_id = MqttString::new("RETAIN_HANDLING_DEFAULT_RECEIVER".into()).unwrap();
    let mut rx = assert_ok!(
        connected_client(
            BROKER_ADDRESS,
            NO_SESSION_CONNECT_OPTIONS,
            Some(rx_id.clone())
        )
        .await
    );

    let pub_options = PublicationOptions {
        retain: true,
        topic: topic.clone(),
        qos: QoS::AtLeastOnce,
    };
    assert_published!(tx, pub_options, msg.into());

    sleep(Duration::from_secs(1)).await;

    // Subscribe with RetainHandling::AlwaysSend (default) - should receive retained message
    let mut options = DEFAULT_QOS0_SUB_OPTIONS;
    options.qos = QoS::AtLeastOnce;
    options.retain_handling = RetainHandling::AlwaysSend;
    assert_subscribe!(rx, options, topic.clone());

    let publish = assert_recv!(rx);
    assert_eq!(&*publish.message, msg.as_bytes());
    assert!(publish.retain);

    // Subscribe again - should receive retained message again with RetainHandling::AlwaysSend
    assert_subscribe!(rx, options, topic.clone());

    let publish = assert_recv!(rx);
    assert_eq!(&*publish.message, msg.as_bytes());
    assert!(publish.retain);

    // Disconnect and reconnect - should receive retained message again
    disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    sleep(Duration::from_secs(1)).await;
    assert_ok!(
        rx.connect(
            Tcp::new(assert_ok!(TcpStream::connect(BROKER_ADDRESS).await)),
            NO_SESSION_CONNECT_OPTIONS,
            Some(rx_id),
        )
        .await
    );
    assert_subscribe!(rx, options, topic.clone());

    let publish = assert_recv!(rx);
    assert_eq!(&*publish.message, msg.as_bytes());
    assert!(publish.retain);

    disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn subscribe_retain_handling_never() {
    let topic = unique_topic();
    let msg = "Retained message for NeverSend.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let rx_id = MqttString::new("RETAIN_HANDLING_NEVER_RECEIVER".into()).unwrap();
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, Some(rx_id)).await);

    let pub_options = PublicationOptions {
        retain: true,
        topic: topic.clone(),
        qos: QoS::AtLeastOnce,
    };
    assert_published!(tx, pub_options, msg.into());

    sleep(Duration::from_secs(1)).await;

    // Subscribe with NeverSend - should NOT receive retained message
    let mut options = DEFAULT_QOS0_SUB_OPTIONS;
    options.qos = QoS::AtLeastOnce;
    options.retain_handling = RetainHandling::NeverSend;
    assert_subscribe!(rx, options, topic.clone());

    assert_err!(
        timeout(Duration::from_secs(5), async {
            assert_recv!(rx);
        })
        .await,
        "Expected to receive nothing with NeverSend"
    );

    // Subscribe again - should still NOT receive retained message
    assert_subscribe!(rx, options, topic.clone());

    assert_err!(
        timeout(Duration::from_secs(5), async {
            assert_recv!(rx);
        })
        .await,
        "Expected to receive nothing on resubscribe with NeverSend"
    );

    disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn subscribe_retain_handling_clean_only() {
    let topic = unique_topic();
    let msg = "Retained message for SendIfNotSubscribedBefore.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let rx_id = MqttString::new("RETAIN_HANDLING_CLEAN_ONLY_RECEIVER".into()).unwrap();
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, Some(rx_id)).await);

    let pub_options = PublicationOptions {
        retain: true,
        topic: topic.clone(),
        qos: QoS::AtLeastOnce,
    };
    assert_published!(tx, pub_options, msg.into());

    sleep(Duration::from_secs(1)).await;

    // Subscribe for the first time with RetainHandling::SendIfNotSubscribedBefore - should receive retained message
    let mut options = DEFAULT_QOS0_SUB_OPTIONS;
    options.qos = QoS::AtLeastOnce;
    options.retain_handling = RetainHandling::SendIfNotSubscribedBefore;
    assert_subscribe!(rx, options, topic.clone());

    let publish = assert_recv!(rx);
    assert_eq!(&*publish.message, msg.as_bytes());
    assert!(publish.retain);

    // Subscribe again - should NOT receive retained message (already subscribed before)
    assert_subscribe!(rx, options, topic.clone());

    assert_err!(
        timeout(Duration::from_secs(5), async {
            assert_recv!(rx);
        })
        .await,
        "Expected to receive nothing on resubscribe with RetainHandling::SendIfNotSubscribedBefore"
    );

    disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn subscribe_retain_as_published_false() {
    let topic = unique_topic();
    let msg = "Retained message for SendIfNotSubscribedBefore.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let pub_options = PublicationOptions {
        retain: true,
        topic: topic.clone(),
        qos: QoS::AtLeastOnce,
    };
    assert_published!(tx, pub_options.clone(), msg.into());

    sleep(Duration::from_secs(1)).await;

    let options = DEFAULT_QOS0_SUB_OPTIONS;
    assert_subscribe!(rx, options, topic.clone());

    let publish = assert_recv!(rx);
    assert_eq!(&*publish.message, msg.as_bytes());
    assert!(
        publish.retain,
        "Retain flag is always set to true if the message was retained and delivered because of new subscription"
    );

    assert_published!(tx, pub_options, msg.into());

    let publish = assert_recv!(rx);
    assert_eq!(&*publish.message, msg.as_bytes());
    assert!(!publish.retain, "Retain flag should be set to false");

    disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn subscribe_retain_as_published_true() {
    let topic = unique_topic();
    let msg = "Retained message for SendIfNotSubscribedBefore.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let pub_options = PublicationOptions {
        retain: true,
        topic: topic.clone(),
        qos: QoS::AtLeastOnce,
    };
    assert_published!(tx, pub_options.clone(), msg.into());

    sleep(Duration::from_secs(1)).await;

    let mut options = DEFAULT_QOS0_SUB_OPTIONS;
    options.retain_as_published = true;
    assert_subscribe!(rx, options, topic.clone());

    let publish = assert_recv!(rx);
    assert_eq!(&*publish.message, msg.as_bytes());
    assert!(
        publish.retain,
        "Retain flag is always set to true if the message was retained and delivered because of new subscription"
    );

    assert_published!(tx, pub_options, msg.into());

    let publish = assert_recv!(rx);
    assert_eq!(&*publish.message, msg.as_bytes());
    assert!(publish.retain, "Retain flag should be set to true");

    disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
}
