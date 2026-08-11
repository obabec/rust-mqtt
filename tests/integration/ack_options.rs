use std::{assert_eq, matches, panic, time::Duration};

use rust_mqtt::{
    client::{
        event::{Event, Publish},
        options::{AckOptions, PublicationOptions, TopicReference},
    },
    types::{IdentifiedQoS, MqttString, MqttStringPair, ReasonCode},
};
use tokio::{join, time::sleep};

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, DEFAULT_QOS0_SUB_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::{assert_ok, assert_published, assert_subscribe},
    utils::{connected_client, disconnect, unique_topic},
};

#[tokio::test]
#[test_log::test]
async fn puback_user_properties() {
    let user_properties: [_; 16] = std::array::from_fn(|i| {
        MqttStringPair::new(
            MqttString::try_from(format!("key_{i}")).unwrap(),
            MqttString::try_from(format!("value_{i}")).unwrap(),
        )
    });

    let (topic_name, topic_filter) = unique_topic();
    let msg = "Deleted code is debugged code.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).at_least_once();
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            &DEFAULT_QOS0_SUB_OPTIONS.at_least_once(),
            topic_filter.clone()
        );

        let Event::Publish(Publish {
            identified_qos: IdentifiedQoS::AtLeastOnce(pid),
            ..
        }) = assert_ok!(rx.poll().await)
        else {
            panic!()
        };

        assert_ok!(
            rx.manual_acknowledge(
                pid,
                ReasonCode::Success,
                &AckOptions::new().user_properties(&user_properties),
            )
            .await
        );

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[should_panic]
#[tokio::test]
#[test_log::test]
async fn puback_too_many_user_properties() {
    let user_properties: [_; 17] = std::array::from_fn(|i| {
        MqttStringPair::new(
            MqttString::try_from(format!("key_{i}")).unwrap(),
            MqttString::try_from(format!("value_{i}")).unwrap(),
        )
    });

    let (topic_name, topic_filter) = unique_topic();
    let msg = "Software and cathedrals are much the same—first we build them, then we pray.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).at_least_once();
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            &DEFAULT_QOS0_SUB_OPTIONS.at_least_once(),
            topic_filter.clone()
        );

        let Event::Publish(Publish {
            identified_qos: IdentifiedQoS::AtLeastOnce(pid),
            ..
        }) = assert_ok!(rx.poll().await)
        else {
            panic!()
        };

        let _ = rx
            .manual_acknowledge(
                pid,
                ReasonCode::Success,
                &AckOptions::new().user_properties(&user_properties),
            )
            .await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn pubrec_pubcomp_user_properties() {
    let user_properties: [_; 16] = std::array::from_fn(|i| {
        MqttStringPair::new(
            MqttString::try_from(format!("key_{i}")).unwrap(),
            MqttString::try_from(format!("value_{i}")).unwrap(),
        )
    });

    let (topic_name, topic_filter) = unique_topic();
    let msg = "Experience is the name everyone gives to their mistakes.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            &DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            topic_filter.clone()
        );

        let Event::Publish(Publish {
            identified_qos: IdentifiedQoS::ExactlyOnce(pid),
            ..
        }) = assert_ok!(rx.poll().await)
        else {
            panic!()
        };

        assert_ok!(
            rx.manual_receive(
                pid,
                ReasonCode::Success,
                &AckOptions::new().user_properties(&user_properties),
            )
            .await
        );

        let e = assert_ok!(rx.poll().await);
        assert!(matches!(e, Event::PublishReleased(_)));

        assert_ok!(
            rx.manual_complete(pid, &AckOptions::new().user_properties(&user_properties),)
                .await
        );

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[should_panic]
#[tokio::test]
#[test_log::test]
async fn pubrec_too_many_user_properties() {
    let user_properties: [_; 17] = std::array::from_fn(|i| {
        MqttStringPair::new(
            MqttString::try_from(format!("key_{i}")).unwrap(),
            MqttString::try_from(format!("value_{i}")).unwrap(),
        )
    });

    let (topic_name, topic_filter) = unique_topic();
    let msg = "The problem with troubleshooting is that trouble shoots back.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            &DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            topic_filter.clone()
        );

        let Event::Publish(Publish {
            identified_qos: IdentifiedQoS::ExactlyOnce(pid),
            ..
        }) = assert_ok!(rx.poll().await)
        else {
            panic!()
        };

        let _ = rx
            .manual_receive(
                pid,
                ReasonCode::Success,
                &AckOptions::new().user_properties(&user_properties),
            )
            .await;
    };

    join!(receiver, publisher);
}

#[should_panic]
#[tokio::test]
#[test_log::test]
async fn pubcomp_too_many_user_properties() {
    let user_properties: [_; 17] = std::array::from_fn(|i| {
        MqttStringPair::new(
            MqttString::try_from(format!("key_{i}")).unwrap(),
            MqttString::try_from(format!("value_{i}")).unwrap(),
        )
    });

    let (topic_name, topic_filter) = unique_topic();
    let msg = "To err is human, to really foul things up requires a computer.";

    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        sleep(Duration::from_secs(1)).await;

        let pub_options =
            PublicationOptions::new(TopicReference::Name(topic_name.clone())).exactly_once();
        assert_published!(tx, pub_options, msg.into());

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            &DEFAULT_QOS0_SUB_OPTIONS.exactly_once(),
            topic_filter.clone()
        );

        let Event::Publish(Publish {
            identified_qos: IdentifiedQoS::ExactlyOnce(pid),
            ..
        }) = assert_ok!(rx.poll().await)
        else {
            panic!()
        };

        assert_ok!(
            rx.manual_receive(pid, ReasonCode::Success, &AckOptions::new(),)
                .await
        );

        let e = assert_ok!(rx.poll().await);
        assert!(matches!(e, Event::PublishReleased(_)));

        let _ = rx
            .manual_complete(pid, &AckOptions::new().user_properties(&user_properties))
            .await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn pubrel_user_properties() {
    let user_properties: [_; 16] = std::array::from_fn(|i| {
        MqttStringPair::new(
            MqttString::try_from(format!("key_{i}")).unwrap(),
            MqttString::try_from(format!("value_{i}")).unwrap(),
        )
    });

    let (topic_name, _) = unique_topic();
    let msg = "The only thing permanent is a temporary workaround.";

    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()))
        .exactly_once()
        .ack_manually();

    let pid = assert_ok!(c.publish(&pub_options, msg.into()).await).unwrap();

    let e = assert_ok!(c.poll().await);
    assert!(matches!(e, Event::PublishReceived(_)));

    assert_ok!(
        c.manual_release(pid, &AckOptions::new().user_properties(&user_properties))
            .await
    );

    let e = assert_ok!(c.poll().await);
    assert!(matches!(e, Event::PublishComplete(_)));

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[should_panic]
#[tokio::test]
#[test_log::test]
async fn pubrel_too_many_user_properties() {
    let user_properties: [_; 17] = std::array::from_fn(|i| {
        MqttStringPair::new(
            MqttString::try_from(format!("key_{i}")).unwrap(),
            MqttString::try_from(format!("value_{i}")).unwrap(),
        )
    });

    let (topic_name, _) = unique_topic();
    let msg = "Yesterday it worked. Today it is a feature.";

    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()))
        .exactly_once()
        .ack_manually();

    let pid = assert_ok!(c.publish(&pub_options, msg.into()).await).unwrap();

    let e = assert_ok!(c.poll().await);
    assert!(matches!(e, Event::PublishReceived(_)));

    assert_ok!(
        c.manual_release(pid, &AckOptions::new().user_properties(&user_properties))
            .await
    );
}
