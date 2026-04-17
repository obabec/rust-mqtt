use std::{num::NonZero, time::Duration};

use rust_mqtt::{
    client::{
        event::Publish,
        options::{PublicationOptions, TopicReference},
    },
    types::{MqttString, MqttStringPair},
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
        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()))
            .at_least_once()
            .message_expiry_interval(10);

        sleep(Duration::from_secs(5)).await;
        assert_published!(tx, pub_options.clone(), msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let options = DEFAULT_QOS0_SUB_OPTIONS.at_least_once();
        assert_subscribe!(rx, &options, topic_filter.clone());

        let Publish {
            message_expiry_interval,
            message,
            ..
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
        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()))
            .retain()
            .message_expiry_interval(10)
            .at_least_once();

        assert_published!(tx, pub_options.clone(), msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        sleep(Duration::from_secs(5)).await;

        let options = DEFAULT_QOS0_SUB_OPTIONS.at_least_once();
        assert_subscribe!(rx, &options, topic_filter.clone());

        let Publish {
            message_expiry_interval,
            message,
            ..
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
        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()))
            .retain()
            .message_expiry_interval(5)
            .at_least_once();

        assert_published!(tx, pub_options.clone(), msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        sleep(Duration::from_secs(6)).await;

        let options = DEFAULT_QOS0_SUB_OPTIONS.at_least_once();
        assert_subscribe!(rx, &options, topic_filter.clone());

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
    let msg = "There are two ways to write error-free programs. Only the third one works.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions::new(TopicReference::Mapping(
            topic_name.clone(),
            NonZero::new(1).unwrap(),
        ))
        .retain()
        .at_least_once();

        assert_published!(tx, pub_options.clone(), msg.into());

        let pub_options = PublicationOptions::new(TopicReference::Alias(NonZero::new(1).unwrap()))
            .retain()
            .at_least_once();

        assert_published!(tx, pub_options.clone(), msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let options = DEFAULT_QOS0_SUB_OPTIONS.at_least_once();
        assert_subscribe!(rx, &options, topic_filter.clone());

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

        let pub_options = PublicationOptions::new(TopicReference::Mapping(
            topic_name1.clone(),
            NonZero::new(1).unwrap(),
        ))
        .retain()
        .at_least_once();

        assert_published!(tx, pub_options.clone(), msg.into());

        let pub_options = PublicationOptions::new(TopicReference::Mapping(
            topic_name2.clone(),
            NonZero::new(1).unwrap(),
        ))
        .retain()
        .at_least_once();

        assert_published!(tx, pub_options.clone(), msg.into());

        let pub_options = PublicationOptions::new(TopicReference::Alias(NonZero::new(1).unwrap()))
            .retain()
            .at_least_once();

        assert_published!(tx, pub_options.clone(), msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver1 = async {
        let options = DEFAULT_QOS0_SUB_OPTIONS.at_least_once();
        assert_subscribe!(rx1, &options, topic_filter1.clone());

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
        let options = DEFAULT_QOS0_SUB_OPTIONS.at_least_once();
        assert_subscribe!(rx2, &options, topic_filter2.clone());

        assert_recv!(rx2);
        assert_recv!(rx2);

        disconnect(&mut rx2, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver1, receiver2, publisher);
}

#[tokio::test]
#[test_log::test]
async fn payload_format_indicator() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "Should array indices start at 0 or 1? My compromise of 0.5 was rejected without, I thought, proper consideration.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()));
        assert_published!(tx, pub_options, msg.into());
        let pub_options = pub_options.payload_format_indicator(false);
        assert_published!(tx, pub_options, msg.into());
        let pub_options = pub_options.payload_format_indicator(true);
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, topic_filter.clone());

        let Publish {
            payload_format_indicator,
            ..
        } = assert_recv_excl!(rx, topic_name);
        assert_eq!(payload_format_indicator, None);

        let Publish {
            payload_format_indicator,
            ..
        } = assert_recv_excl!(rx, topic_name);
        assert_eq!(payload_format_indicator, Some(false));

        let Publish {
            payload_format_indicator,
            ..
        } = assert_recv_excl!(rx, topic_name);
        assert_eq!(payload_format_indicator, Some(true));

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn user_properties() {
    let publish_user_properties = vec![
        MqttStringPair::new(
            MqttString::from_str("last_will").unwrap(),
            MqttString::from_str("delete_my_browser_history").unwrap(),
        ),
        MqttStringPair::new(
            MqttString::from_str("qos_level").unwrap(),
            MqttString::from_str("thoughts_and_prayers").unwrap(),
        ),
        MqttStringPair::new(
            MqttString::from_str("retry_strategy").unwrap(),
            MqttString::from_str("aggressive_procrastination").unwrap(),
        ),
        MqttStringPair::new(
            MqttString::from_str("latency_source").unwrap(),
            MqttString::from_str("cat_on_the_router").unwrap(),
        ),
        MqttStringPair::new(
            MqttString::from_str("link_type").unwrap(),
            MqttString::from_str("carrier_pigeon_with_rfc_1149").unwrap(),
        ),
    ];
    let (topic_name, topic_filter) = unique_topic();
    let msg = "The good thing about reinventing the wheel is that you can get a round one.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()));
        assert_published!(tx, pub_options, msg.into());

        let pub_options = pub_options.user_properties(&publish_user_properties);
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, topic_filter.clone());

        let Publish {
            user_properties, ..
        } = assert_recv_excl!(rx, topic_name);
        assert_eq!(user_properties, &[]);

        let Publish {
            user_properties, ..
        } = assert_recv_excl!(rx, topic_name);

        // The Server MUST maintain the order of User Properties when forwarding the Application Message [MQTT-3.3.2-18]
        assert_eq!(
            user_properties.as_slice(),
            publish_user_properties.as_slice()
        );

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn content_type() {
    let publish_content_type = MqttString::from_str("application/octet-stream").unwrap();
    let (topic_name, topic_filter) = unique_topic();
    let msg = "The good thing about reinventing the wheel is that you can get a round one.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()));
        assert_published!(tx, pub_options, msg.into());

        let pub_options = pub_options.content_type(publish_content_type.as_borrowed());
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, topic_filter.clone());

        let Publish { content_type, .. } = assert_recv_excl!(rx, topic_name);
        assert_eq!(content_type, None);

        let Publish { content_type, .. } = assert_recv_excl!(rx, topic_name);
        assert_eq!(content_type, Some(publish_content_type.as_borrowed()));

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}
