use log::info;
use rust_mqtt::{
    client::options::{PublicationOptions, SubscriptionOptions},
    types::{QoS, TopicName},
};
use tokio::{
    join,
    sync::oneshot::{Receiver, Sender, channel},
};

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::{assert_ok, assert_published, assert_recv, assert_subscribe},
    utils::{connected_client, disconnect, unique_topic},
};

const MSG: &str = "testMessage";

async fn publish_multiple(
    topic: TopicName<'_>,
    qos: QoS,
    count: u16,
    ready_rx: Receiver<()>,
) -> Result<(), ()> {
    let mut client =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    info!("[Publisher] Waiting for receiver to be ready");
    let _ = ready_rx.await;
    info!("[Publisher] Receiver is ready, starting to publish");

    let pub_options = PublicationOptions {
        retain: false,
        topic: topic.clone(),
        qos,
    };

    let topic_name = topic.as_ref();
    info!(
        "[Publisher] Sending {} messages to topic {:?}",
        count, topic_name
    );
    for i in 0..count {
        assert_published!(client, pub_options, MSG.into());
        if (i + 1) % 100 == 0 {
            info!("[Publisher] Sent {}/{} messages", i + 1, count);
        }
    }

    info!("[Publisher] Disconnecting after sending {} messages", count);
    disconnect(&mut client, DEFAULT_DC_OPTIONS).await;
    Ok(())
}

async fn receive_multiple(
    topic_name: TopicName<'static>,
    qos: QoS,
    count: u16,
    ready_tx: Sender<()>,
) -> Result<(), ()> {
    let mut client =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let options = SubscriptionOptions {
        retain_handling: rust_mqtt::client::options::RetainHandling::AlwaysSend,
        retain_as_published: false,
        no_local: false,
        qos,
    };

    info!("[Receiver] Subscribing to topic {:?}", topic_name.as_ref());
    assert_subscribe!(client, options, topic_name.into());

    info!("[Receiver] Subscription confirmed, signaling ready");
    let _ = ready_tx.send(());

    info!("[Receiver] Waiting for {} messages", count);
    for i in 0..count {
        let publish = assert_recv!(client);
        assert_eq!(&*publish.message, MSG.as_bytes());
        assert_eq!(publish.qos, qos);

        if (i + 1) % 100 == 0 {
            info!("[Receiver] Received {}/{} messages", i + 1, count);
        }
    }

    info!(
        "[Receiver] Disconnecting after receiving {} messages",
        count
    );
    disconnect(&mut client, DEFAULT_DC_OPTIONS).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_ten_qos0() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::AtMostOnce, 10, ready_tx),
        publish_multiple(topic_name, QoS::AtMostOnce, 10, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_ten_qos1() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::AtLeastOnce, 10, ready_tx),
        publish_multiple(topic_name, QoS::AtLeastOnce, 10, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_ten_qos2() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::ExactlyOnce, 10, ready_tx),
        publish_multiple(topic_name, QoS::ExactlyOnce, 10, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_fifty_qos0() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::AtMostOnce, 50, ready_tx),
        publish_multiple(topic_name, QoS::AtMostOnce, 50, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_fifty_qos1() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::AtLeastOnce, 50, ready_tx),
        publish_multiple(topic_name, QoS::AtLeastOnce, 50, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_fifty_qos2() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::ExactlyOnce, 50, ready_tx),
        publish_multiple(topic_name, QoS::ExactlyOnce, 50, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_five_hundred_qos0() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::AtMostOnce, 500, ready_tx),
        publish_multiple(topic_name, QoS::AtMostOnce, 500, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_five_hundred_qos1() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::AtLeastOnce, 500, ready_tx),
        publish_multiple(topic_name, QoS::AtLeastOnce, 500, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_five_hundred_qos2() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::ExactlyOnce, 500, ready_tx),
        publish_multiple(topic_name, QoS::ExactlyOnce, 500, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[test_log::test]
async fn load_test_ten_thousand_qos0() {
    let (topic_name, _) = unique_topic();
    let (ready_tx, ready_rx) = channel();

    let (r, p) = join!(
        receive_multiple(topic_name.clone(), QoS::AtMostOnce, 10000, ready_tx),
        publish_multiple(topic_name, QoS::AtMostOnce, 10000, ready_rx)
    );
    assert_ok!(r);
    assert_ok!(p);
}
