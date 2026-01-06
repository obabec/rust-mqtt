use std::time::Duration;

use rust_mqtt::{
    client::{
        event::Publish,
        options::{PublicationOptions, TopicReference},
    },
    types::QoS,
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
async fn message_expiry_interval_basic() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "It's not a bug, it's a forthcoming feature";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: false,
            message_expiry_interval: Some(10),
            topic: TopicReference::Name(topic_name.clone()),
            qos: QoS::AtLeastOnce,
        };

        sleep(Duration::from_secs(5)).await;
        assert_published!(tx, pub_options.clone(), msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;
        assert_subscribe!(rx, options, topic_filter.clone());

        let Publish {
            identified_qos: _,
            dup: _,
            retain: _,
            message_expiry_interval,
            subscription_identifiers: _,
            topic: _,
            message,
        } = assert_recv_excl!(rx, topic_name);

        assert_eq!(&*message, msg.as_bytes());
        assert!(message_expiry_interval.is_some());
        assert!(matches!(message_expiry_interval.unwrap(), 9..=10));
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "enable this test once https://github.com/hivemq/hivemq-community-edition/issues/616 is fixed"]
#[tokio::test]
#[test_log::test]
async fn message_expiry_interval_partially_expired() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "It's not a bug, it's an undocumented feature!";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: true,
            message_expiry_interval: Some(10),
            topic: TopicReference::Name(topic_name.clone()),
            qos: QoS::AtLeastOnce,
        };

        assert_published!(tx, pub_options.clone(), msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        sleep(Duration::from_secs(5)).await;

        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;
        assert_subscribe!(rx, options, topic_filter.clone());

        let Publish {
            identified_qos: _,
            dup: _,
            retain: _,
            message_expiry_interval,
            subscription_identifiers: _,
            topic: _,
            message,
        } = assert_recv_excl!(rx, topic_name);

        assert_eq!(&*message, msg.as_bytes());
        assert!(message_expiry_interval.is_some());
        assert!(matches!(message_expiry_interval.unwrap(), 4..=6));
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn message_expiry_interval_completely_expired() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "It works on my system!";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: true,
            message_expiry_interval: Some(5),
            topic: TopicReference::Name(topic_name.clone()),
            qos: QoS::AtLeastOnce,
        };

        assert_published!(tx, pub_options.clone(), msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        sleep(Duration::from_secs(6)).await;

        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;
        assert_subscribe!(rx, options, topic_filter.clone());

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
async fn topic_alias_basic() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "It's working as designed, will update the requirements accordingly.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions {
            retain: true,
            message_expiry_interval: None,
            topic: TopicReference::Mapping(topic_name.clone(), 1),
            qos: QoS::AtLeastOnce,
        };
        assert_published!(tx, pub_options.clone(), msg.into());

        let pub_options = PublicationOptions {
            retain: true,
            message_expiry_interval: None,
            topic: TopicReference::Alias(1),
            qos: QoS::AtLeastOnce,
        };
        assert_published!(tx, pub_options.clone(), msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;
        assert_subscribe!(rx, options, topic_filter.clone());

        assert_recv!(rx);
        assert_recv!(rx);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn topic_alias_remap() {
    let (topic_name1, topic_filter1) = unique_topic();
    let (topic_name2, topic_filter2) = unique_topic();
    let msg = "It's working as designed, will update the requirements accordingly.";

    let mut rx1 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx2 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions {
            retain: true,
            message_expiry_interval: None,
            topic: TopicReference::Mapping(topic_name1.clone(), 1),
            qos: QoS::AtLeastOnce,
        };
        assert_published!(tx, pub_options.clone(), msg.into());

        let pub_options = PublicationOptions {
            retain: true,
            message_expiry_interval: None,
            topic: TopicReference::Mapping(topic_name2.clone(), 1),
            qos: QoS::AtLeastOnce,
        };
        assert_published!(tx, pub_options.clone(), msg.into());

        let pub_options = PublicationOptions {
            retain: true,
            message_expiry_interval: None,
            topic: TopicReference::Alias(1),
            qos: QoS::AtLeastOnce,
        };
        assert_published!(tx, pub_options.clone(), msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver1 = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;
        assert_subscribe!(rx1, options, topic_filter1.clone());

        assert_recv!(rx1);
        assert_err!(
            timeout(Duration::from_secs(5), async {
                assert_recv!(rx1);
            })
            .await,
            "Expected to receive nothing"
        );

        disconnect(&mut rx1, DEFAULT_DC_OPTIONS).await;
    };

    let receiver2 = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;
        assert_subscribe!(rx2, options, topic_filter2.clone());

        assert_recv!(rx2);
        assert_recv!(rx2);

        disconnect(&mut rx2, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver1, receiver2, publisher);
}
