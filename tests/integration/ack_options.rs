use std::{assert_eq, matches, panic, time::Duration, vec};

use rust_mqtt::{
    client::{
        MqttError,
        event::{Event, Publish},
        options::{AckOptions, PublicationOptions, TopicReference},
    },
    types::{IdentifiedQoS, MqttString, MqttStringPair, ReasonCode},
};
use tokio::{
    join,
    time::{sleep, timeout},
};
use tokio_test::assert_err;

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

#[ignore = "mosquitto has no configurable PUBACK maximum packet size"]
#[tokio::test]
#[test_log::test]
async fn server_maximum_packet_size_not_exceeded_by_puback_hive_only() {
    // fixed header, packet identifier, reason code, property length
    const OVERHEAD: u32 = 4 + 2 + 1 + 3;

    const SERVER_MAX_PACKET_SIZE: u32 = 2_000_000;

    const PROPERTY_SIZE: u32 = SERVER_MAX_PACKET_SIZE - OVERHEAD;

    let user_property = MqttStringPair::new(
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
    );

    const USER_PROPERTY_SIZE: u32 = (MqttString::MAX_LENGTH as u32 + 2) * 2 + 1;
    const REASON_STRING_SIZE: u32 = PROPERTY_SIZE - (USER_PROPERTIES * USER_PROPERTY_SIZE) - 3;

    const USER_PROPERTIES: u32 = PROPERTY_SIZE / USER_PROPERTY_SIZE;

    let user_properties: [_; USER_PROPERTIES as usize] =
        std::array::from_fn(|_| user_property.as_borrowed());
    let reason_string = MqttString::try_from(vec![b'a'; REASON_STRING_SIZE as usize]).unwrap();

    let (topic_name, topic_filter) = unique_topic();
    let msg = "The early bird gets the worm, but the second mouse gets the cheese.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name)).at_least_once();

        sleep(Duration::from_secs(1)).await;
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
                &AckOptions::new()
                    .user_properties(&user_properties)
                    .reason_string(reason_string)
            )
            .await
        );

        assert_err!(timeout(Duration::from_secs(1), rx.poll()).await);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "mosquitto has no configurable PUBACK maximum packet size"]
#[tokio::test]
#[test_log::test]
async fn server_maximum_packet_size_exceeded_by_puback_hive_only() {
    // fixed header, packet identifier, reason code, property length
    const OVERHEAD: u32 = 4 + 2 + 1 + 3;

    const SERVER_MAX_PACKET_SIZE: u32 = 2_000_000;

    const PROPERTY_SIZE: u32 = SERVER_MAX_PACKET_SIZE - OVERHEAD;

    let user_property = MqttStringPair::new(
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
    );

    const USER_PROPERTY_SIZE: u32 = (MqttString::MAX_LENGTH as u32 + 2) * 2 + 1;
    const REASON_STRING_SIZE: u32 = PROPERTY_SIZE - (USER_PROPERTIES * USER_PROPERTY_SIZE) - 3;

    const USER_PROPERTIES: u32 = PROPERTY_SIZE / USER_PROPERTY_SIZE;

    let user_properties: [_; USER_PROPERTIES as usize] =
        std::array::from_fn(|_| user_property.as_borrowed());

    // Exceed the limit
    let reason_string = MqttString::try_from(vec![b'a'; REASON_STRING_SIZE as usize + 1]).unwrap();

    let (topic_name, topic_filter) = unique_topic();
    let msg = "The problem with common sense is that it’s not all that common.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name)).at_least_once();

        sleep(Duration::from_secs(1)).await;
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

        let e = assert_err!(
            rx.manual_acknowledge(
                pid,
                ReasonCode::Success,
                &AckOptions::new()
                    .user_properties(&user_properties)
                    .reason_string(reason_string)
            )
            .await
        );

        assert_eq!(e, MqttError::ServerMaximumPacketSizeExceeded);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "mosquitto has no configurable PUBREC/PUBCOMP maximum packet size"]
#[tokio::test]
#[test_log::test]
async fn server_maximum_packet_size_not_exceeded_by_pubrec_pubcomp_hive_only() {
    // fixed header, packet identifier, reason code, property length
    const OVERHEAD: u32 = 4 + 2 + 1 + 3;

    const SERVER_MAX_PACKET_SIZE: u32 = 2_000_000;

    const PROPERTY_SIZE: u32 = SERVER_MAX_PACKET_SIZE - OVERHEAD;

    let user_property = MqttStringPair::new(
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
    );

    const USER_PROPERTY_SIZE: u32 = (MqttString::MAX_LENGTH as u32 + 2) * 2 + 1;
    const REASON_STRING_SIZE: u32 = PROPERTY_SIZE - (USER_PROPERTIES * USER_PROPERTY_SIZE) - 3;

    const USER_PROPERTIES: u32 = PROPERTY_SIZE / USER_PROPERTY_SIZE;

    let user_properties: [_; USER_PROPERTIES as usize] =
        std::array::from_fn(|_| user_property.as_borrowed());
    let reason_string = MqttString::try_from(vec![b'a'; REASON_STRING_SIZE as usize]).unwrap();

    let (topic_name, topic_filter) = unique_topic();
    let msg = "Experience is a wonderful thing. It enables you to recognize a mistake when you make it again.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name)).exactly_once();

        sleep(Duration::from_secs(1)).await;
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
                &AckOptions::new()
                    .user_properties(&user_properties)
                    .reason_string(reason_string.as_borrowed())
            )
            .await
        );

        let Event::PublishReleased(_) = assert_ok!(rx.poll().await) else {
            panic!()
        };

        assert_ok!(
            rx.manual_complete(
                pid,
                &AckOptions::new()
                    .user_properties(&user_properties)
                    .reason_string(reason_string)
            )
            .await
        );

        assert_err!(timeout(Duration::from_secs(1), rx.poll()).await);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "mosquitto has no configurable PUBREC maximum packet size"]
#[tokio::test]
#[test_log::test]
async fn server_maximum_packet_size_exceeded_by_pubrec_hive_only() {
    // fixed header, packet identifier, reason code, property length
    const OVERHEAD: u32 = 4 + 2 + 1 + 3;

    const SERVER_MAX_PACKET_SIZE: u32 = 2_000_000;

    const PROPERTY_SIZE: u32 = SERVER_MAX_PACKET_SIZE - OVERHEAD;

    let user_property = MqttStringPair::new(
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
    );

    const USER_PROPERTY_SIZE: u32 = (MqttString::MAX_LENGTH as u32 + 2) * 2 + 1;
    const REASON_STRING_SIZE: u32 = PROPERTY_SIZE - (USER_PROPERTIES * USER_PROPERTY_SIZE) - 3;

    const USER_PROPERTIES: u32 = PROPERTY_SIZE / USER_PROPERTY_SIZE;

    let user_properties: [_; USER_PROPERTIES as usize] =
        std::array::from_fn(|_| user_property.as_borrowed());

    // Exceed the limit
    let reason_string = MqttString::try_from(vec![b'a'; REASON_STRING_SIZE as usize + 1]).unwrap();

    let (topic_name, topic_filter) = unique_topic();
    let msg = "This message is 42 characters long. I checked.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name)).exactly_once();

        sleep(Duration::from_secs(1)).await;
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

        let e = assert_err!(
            rx.manual_receive(
                pid,
                ReasonCode::Success,
                &AckOptions::new()
                    .user_properties(&user_properties)
                    .reason_string(reason_string)
            )
            .await
        );

        assert_eq!(e, MqttError::ServerMaximumPacketSizeExceeded);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "mosquitto has no configurable PUBCOMP maximum packet size"]
#[tokio::test]
#[test_log::test]
async fn server_maximum_packet_size_exceeded_by_pubcomp_hive_only() {
    // fixed header, packet identifier, reason code, property length
    const OVERHEAD: u32 = 4 + 2 + 1 + 3;

    const SERVER_MAX_PACKET_SIZE: u32 = 2_000_000;

    const PROPERTY_SIZE: u32 = SERVER_MAX_PACKET_SIZE - OVERHEAD;

    let user_property = MqttStringPair::new(
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
    );

    const USER_PROPERTY_SIZE: u32 = (MqttString::MAX_LENGTH as u32 + 2) * 2 + 1;
    const REASON_STRING_SIZE: u32 = PROPERTY_SIZE - (USER_PROPERTIES * USER_PROPERTY_SIZE) - 3;

    const USER_PROPERTIES: u32 = PROPERTY_SIZE / USER_PROPERTY_SIZE;

    let user_properties: [_; USER_PROPERTIES as usize] =
        std::array::from_fn(|_| user_property.as_borrowed());

    // Exceed the limit
    let reason_string = MqttString::try_from(vec![b'a'; REASON_STRING_SIZE as usize + 1]).unwrap();

    let (topic_name, topic_filter) = unique_topic();
    let msg = "If you are reading this, the bug has gained sentience.";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    rx.ack_manually_when(&|_| true);

    let publisher = async {
        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name)).exactly_once();

        sleep(Duration::from_secs(1)).await;
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
            rx.manual_receive(pid, ReasonCode::Success, &AckOptions::new())
                .await
        );

        let Event::PublishReleased(_) = assert_ok!(rx.poll().await) else {
            panic!()
        };

        let e = assert_err!(
            rx.manual_complete(
                pid,
                &AckOptions::new()
                    .user_properties(&user_properties)
                    .reason_string(reason_string)
            )
            .await
        );

        assert_eq!(e, MqttError::ServerMaximumPacketSizeExceeded);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "mosquitto has no configurable PUBREL maximum packet size"]
#[tokio::test]
#[test_log::test]
async fn server_maximum_packet_size_not_exceeded_by_pubrel_hive_only() {
    // fixed header, packet identifier, reason code, property length
    const OVERHEAD: u32 = 4 + 2 + 1 + 3;

    const SERVER_MAX_PACKET_SIZE: u32 = 2_000_000;

    const PROPERTY_SIZE: u32 = SERVER_MAX_PACKET_SIZE - OVERHEAD;

    let user_property = MqttStringPair::new(
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
    );

    const USER_PROPERTY_SIZE: u32 = (MqttString::MAX_LENGTH as u32 + 2) * 2 + 1;
    const REASON_STRING_SIZE: u32 = PROPERTY_SIZE - (USER_PROPERTIES * USER_PROPERTY_SIZE) - 3;

    const USER_PROPERTIES: u32 = PROPERTY_SIZE / USER_PROPERTY_SIZE;

    let user_properties: [_; USER_PROPERTIES as usize] =
        std::array::from_fn(|_| user_property.as_borrowed());
    let reason_string = MqttString::try_from(vec![b'a'; REASON_STRING_SIZE as usize]).unwrap();

    let (topic_name, _) = unique_topic();
    let msg = "Every solution is just a new problem in a more expensive suit.";

    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let pub_options = PublicationOptions::new(TopicReference::Name(topic_name))
        .exactly_once()
        .ack_manually();

    let pid = assert_ok!(c.publish(&pub_options, msg.into()).await).unwrap();

    let Event::PublishReceived(_) = assert_ok!(c.poll().await) else {
        panic!()
    };

    assert_ok!(
        c.manual_release(
            pid,
            &AckOptions::new()
                .user_properties(&user_properties)
                .reason_string(reason_string.as_borrowed())
        )
        .await
    );

    let Event::PublishComplete(_) = assert_ok!(c.poll().await) else {
        panic!()
    };

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[ignore = "mosquitto has no configurable PUBREL maximum packet size"]
#[tokio::test]
#[test_log::test]
async fn server_maximum_packet_size_exceeded_by_pubrel_hive_only() {
    // fixed header, packet identifier, reason code, property length
    const OVERHEAD: u32 = 4 + 2 + 1 + 3;

    const SERVER_MAX_PACKET_SIZE: u32 = 2_000_000;

    const PROPERTY_SIZE: u32 = SERVER_MAX_PACKET_SIZE - OVERHEAD;

    let user_property = MqttStringPair::new(
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
    );

    const USER_PROPERTY_SIZE: u32 = (MqttString::MAX_LENGTH as u32 + 2) * 2 + 1;
    const REASON_STRING_SIZE: u32 = PROPERTY_SIZE - (USER_PROPERTIES * USER_PROPERTY_SIZE) - 3;

    const USER_PROPERTIES: u32 = PROPERTY_SIZE / USER_PROPERTY_SIZE;

    let user_properties: [_; USER_PROPERTIES as usize] =
        std::array::from_fn(|_| user_property.as_borrowed());

    // Exceed the limit
    let reason_string = MqttString::try_from(vec![b'a'; REASON_STRING_SIZE as usize + 1]).unwrap();

    let (topic_name, _) = unique_topic();
    let msg = "Efficiency is a highly developed form of laziness.";

    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let pub_options = PublicationOptions::new(TopicReference::Name(topic_name))
        .exactly_once()
        .ack_manually();

    let pid = assert_ok!(c.publish(&pub_options, msg.into()).await).unwrap();

    let Event::PublishReceived(_) = assert_ok!(c.poll().await) else {
        panic!()
    };

    let e = assert_err!(
        c.manual_release(
            pid,
            &AckOptions::new()
                .user_properties(&user_properties)
                .reason_string(reason_string.as_borrowed())
        )
        .await
    );
    assert_eq!(e, MqttError::ServerMaximumPacketSizeExceeded);

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}
