use std::time::Duration;

use rust_mqtt::{
    Bytes,
    client::{
        event::Publish,
        options::{PublicationOptions, TopicReference},
    },
    types::MqttBinary,
};
use tokio::{
    join,
    time::{sleep, timeout},
};

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, DEFAULT_QOS0_SUB_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::{assert_ok, assert_published, assert_recv_excl, assert_subscribe},
    utils::{connected_client, disconnect, receive_and_complete, unique_topic},
};

#[tokio::test]
#[test_log::test]
async fn simple_request_response() {
    let (topic_name, topic_filter) = unique_topic();
    let (response_topic_name, response_topic_filter) = unique_topic();
    let request_msg = "Hey";
    let response_msg = "Hi!";

    let mut requester =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut responder =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let request = async {
        assert_subscribe!(requester, DEFAULT_QOS0_SUB_OPTIONS, response_topic_filter);

        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()))
            .response_topic(response_topic_name.clone());

        assert_published!(requester, pub_options, request_msg.into());

        let Publish {
            response_topic,
            correlation_data,
            topic,
            message,
            ..
        } = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(5), receive_and_complete(&mut requester)).await
        ));

        assert_eq!(topic, response_topic_name);
        assert_eq!(&*message, response_msg.as_bytes());
        assert!(response_topic.is_none());
        assert!(correlation_data.is_none());

        disconnect(&mut requester, DEFAULT_DC_OPTIONS).await;
    };

    let response = async {
        assert_subscribe!(responder, DEFAULT_QOS0_SUB_OPTIONS, topic_filter);

        let Publish {
            response_topic,
            correlation_data,
            topic,
            message,
            ..
        } = assert_recv_excl!(responder, topic_name);

        assert_eq!(topic, topic_name);
        assert_eq!(&*message, request_msg.as_bytes());
        assert!(response_topic.is_some());
        assert_eq!(response_topic.as_ref().unwrap(), &response_topic_name);
        assert!(correlation_data.is_none());

        let pub_options = PublicationOptions::new(TopicReference::Name(response_topic.unwrap()));
        assert_published!(responder, pub_options, response_msg.into());

        disconnect(&mut responder, DEFAULT_DC_OPTIONS).await;
    };

    join!(request, response);
}

#[tokio::test]
#[test_log::test]
async fn simple_correlation_data() {
    let (topic_name, topic_filter) = unique_topic();
    let (response_topic_name, response_topic_filter) = unique_topic();
    let request_msg = "Hey";
    let response_msg = "Hi!";
    let correlation = MqttBinary::from_slice(&[
        2, 3, 6, 66, 1, 51, 29, 91, 194, 245, 123, 84, 123, 19, 35, 6, 61, 4, 1,
    ])
    .unwrap();

    let mut requester =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut responder =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let request = async {
        assert_subscribe!(requester, DEFAULT_QOS0_SUB_OPTIONS, response_topic_filter);

        sleep(Duration::from_secs(1)).await;

        let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()))
            .response_topic(response_topic_name.clone())
            .correlation_data(correlation.clone());

        assert_published!(requester, pub_options, request_msg.into());

        let Publish {
            response_topic,
            correlation_data,
            topic,
            message,
            ..
        } = assert_ok!(assert_ok!(
            timeout(Duration::from_secs(5), receive_and_complete(&mut requester)).await
        ));

        assert_eq!(topic, response_topic_name);
        assert_eq!(&*message, response_msg.as_bytes());
        assert!(response_topic.is_none());
        assert!(correlation_data.is_some());
        assert_eq!(correlation_data.unwrap(), correlation);

        disconnect(&mut requester, DEFAULT_DC_OPTIONS).await;
    };

    let response = async {
        assert_subscribe!(responder, DEFAULT_QOS0_SUB_OPTIONS, topic_filter);

        let Publish {
            response_topic,
            correlation_data,
            topic,
            message,
            ..
        } = assert_recv_excl!(responder, topic_name);

        assert_eq!(topic, topic_name);
        assert_eq!(&*message, request_msg.as_bytes());
        assert!(response_topic.is_some());
        assert_eq!(response_topic.as_ref().unwrap(), &response_topic_name);
        assert!(correlation_data.is_some());
        assert_eq!(correlation_data.as_ref().unwrap(), &correlation);

        let pub_options = PublicationOptions::new(TopicReference::Name(response_topic.unwrap()))
            .correlation_data(correlation_data.unwrap());
        assert_published!(responder, pub_options, response_msg.into());

        disconnect(&mut responder, DEFAULT_DC_OPTIONS).await;
    };

    join!(request, response);
}

#[tokio::test]
#[test_log::test]
async fn multiple_correlation_data() {
    let (topic_name, topic_filter) = unique_topic();
    let (response_topic_name, response_topic_filter) = unique_topic();
    let request_msg = Bytes::Borrowed(b"slice");
    let correlations = vec![
        MqttBinary::from_slice(&[2, 3, 6, 66, 1, 51, 29]).unwrap(),
        MqttBinary::from_slice(&[91, 194, 245, 123, 84, 123]).unwrap(),
        MqttBinary::from_slice(&[19, 35, 6, 61, 4, 1]).unwrap(),
        MqttBinary::from_slice(&[10, 20, 30, 40, 50]).unwrap(),
        MqttBinary::from_slice(&[7, 8, 9, 10, 11, 12]).unwrap(),
        MqttBinary::from_slice(&[200, 201, 202, 203]).unwrap(),
        MqttBinary::from_slice(&[99, 100, 101, 102, 103, 104, 105]).unwrap(),
        MqttBinary::from_slice(&[1, 4, 9, 16, 25, 36]).unwrap(),
    ];

    let mut requester =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut responder =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let request = async {
        assert_subscribe!(requester, DEFAULT_QOS0_SUB_OPTIONS, response_topic_filter);

        sleep(Duration::from_secs(1)).await;

        for correlation in correlations.iter() {
            let pub_options = PublicationOptions::new(TopicReference::Name(topic_name.clone()))
                .response_topic(response_topic_name.clone())
                .correlation_data(correlation.clone());

            assert_published!(requester, pub_options, request_msg.clone());
        }

        let mut expected = correlations.clone();
        for _ in 0..expected.len() {
            let Publish {
                response_topic,
                correlation_data,
                topic,
                message,
                ..
            } = assert_ok!(assert_ok!(
                timeout(Duration::from_secs(5), receive_and_complete(&mut requester)).await
            ));

            assert_eq!(topic, response_topic_name);
            assert!(response_topic.is_none());

            let correlation = correlation_data.expect("Expected correlation data");
            assert_eq!(&*message, correlation.as_ref());

            let pos = expected
                .iter()
                .position(|expected_correlation| expected_correlation == &correlation)
                .expect("Unexpected correlation data");
            expected.remove(pos);
        }

        disconnect(&mut requester, DEFAULT_DC_OPTIONS).await;
    };

    let response = async {
        assert_subscribe!(responder, DEFAULT_QOS0_SUB_OPTIONS, topic_filter);

        for _ in 0..correlations.len() {
            let Publish {
                response_topic,
                correlation_data,
                topic,
                ..
            } = assert_recv_excl!(responder, topic_name);

            assert_eq!(topic, topic_name);
            assert!(response_topic.is_some());
            assert_eq!(response_topic.as_ref().unwrap(), &response_topic_name);

            let correlation = correlation_data.expect("Expected correlation data");

            let pub_options =
                PublicationOptions::new(TopicReference::Name(response_topic.unwrap()))
                    .correlation_data(correlation.clone());
            assert_published!(responder, pub_options, correlation.as_ref().into());
        }

        disconnect(&mut responder, DEFAULT_DC_OPTIONS).await;
    };

    join!(request, response);
}
