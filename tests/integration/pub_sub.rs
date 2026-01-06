use std::time::Duration;

use log::info;
use rust_mqtt::{
    client::{event::Publish, options::PublicationOptions},
    types::{IdentifiedQoS, QoS},
};
use tokio::{
    join,
    time::{sleep, timeout},
};
use tokio_test::assert_err;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, DEFAULT_QOS0_SUB_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::{
        assert_ok, assert_published, assert_recv, assert_recv_excl, assert_subscribe,
        assert_unsubscribe,
    },
    utils::{connected_client, disconnect, unique_topic},
};

#[tokio::test]
#[test_log::test]
async fn publish_recv_qos0() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "test message";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name.clone(),
            qos: QoS::AtMostOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, topic_filter.clone());
        let Publish {
            identified_qos,
            dup,
            retain,
            subscription_identifiers: _,
            topic: _,
            message,
        } = assert_recv_excl!(rx, topic_filter);

        assert!(!dup);
        assert!(!retain);
        assert_eq!(
            <IdentifiedQoS as Into<QoS>>::into(identified_qos),
            QoS::AtMostOnce
        );
        assert_eq!(&*message, msg.as_bytes());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn publish_recv_qos1() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "lorem ipsum";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name.clone(),
            qos: QoS::AtLeastOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;

        assert_subscribe!(rx, options, topic_filter.clone());
        let Publish {
            identified_qos,
            dup,
            retain,
            subscription_identifiers: _,
            topic: _,
            message,
        } = assert_recv_excl!(rx, topic_name);

        assert!(!dup);
        assert!(!retain);
        assert_eq!(
            <IdentifiedQoS as Into<QoS>>::into(identified_qos),
            QoS::AtLeastOnce
        );
        assert_eq!(&*message, msg.as_bytes());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn publish_recv_qos2() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "01001000 01101001 (Hi in binary)";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name.clone(),
            qos: QoS::ExactlyOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::ExactlyOnce;

        assert_subscribe!(rx, options, topic_filter.clone());
        let Publish {
            identified_qos,
            dup,
            retain,
            subscription_identifiers: _,
            topic: _,
            message,
        } = assert_recv_excl!(rx, topic_name);

        assert!(!dup);
        assert!(!retain);
        assert_eq!(
            <IdentifiedQoS as Into<QoS>>::into(identified_qos),
            QoS::ExactlyOnce
        );
        assert_eq!(&*message, msg.as_bytes());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn publish_recv_multiple_qos0() {
    let (topic_name1, topic_filter1) = unique_topic();
    let (topic_name2, topic_filter2) = unique_topic();
    let msg = "418 I'm a teapot.";

    let mut tx1 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx2 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher1 = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name1.clone(),
            qos: QoS::AtMostOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx1, pub_options, msg.into());
        disconnect(&mut tx1, DEFAULT_DC_OPTIONS).await;
    };
    let publisher2 = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name2.clone(),
            qos: QoS::AtMostOnce,
        };

        sleep(Duration::from_secs(2)).await;
        assert_published!(tx2, pub_options, msg.into());
        disconnect(&mut tx2, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, topic_filter1.clone());
        assert_subscribe!(rx, DEFAULT_QOS0_SUB_OPTIONS, topic_filter2.clone());

        let mut messages = vec![(topic_filter1, msg), (topic_filter2, msg)];

        let _ = timeout(Duration::from_secs(5), async {
            while !messages.is_empty() {
                let p = assert_recv!(rx);

                let matching_msg = assert_ok!(
                    messages
                        .iter()
                        .enumerate()
                        .find(|(_, (t, m))| *t.as_ref() == p.topic && m == &msg)
                        .ok_or("Option is None")
                );

                messages.remove(matching_msg.0);
            }
        })
        .await;

        info!("{:?}", messages);
        assert!(messages.is_empty());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher1, publisher2);
}

#[tokio::test]
#[test_log::test]
async fn publish_recv_multiple_qos1() {
    let (topic_name1, topic_filter1) = unique_topic();
    let (topic_name2, topic_filter2) = unique_topic();
    let msg = "= note: Send would have to be implemented for the type MqttBinary<'_> \n\
                     = note: ...but Send is actually implemented for the type MqttBinary<'0>, for some specific lifetime '0";

    let mut tx1 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx2 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher1 = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name1.clone(),
            qos: QoS::AtLeastOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx1, pub_options, msg.into());
        disconnect(&mut tx1, DEFAULT_DC_OPTIONS).await;
    };
    let publisher2 = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name2.clone(),
            qos: QoS::AtLeastOnce,
        };

        sleep(Duration::from_secs(2)).await;
        assert_published!(tx2, pub_options, msg.into());
        disconnect(&mut tx2, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;

        assert_subscribe!(rx, options, topic_filter1.clone());
        assert_subscribe!(rx, options, topic_filter2.clone());

        let mut messages = vec![(topic_name1.clone(), msg), (topic_name2.clone(), msg)];

        let _ = timeout(Duration::from_secs(5), async {
            while !messages.is_empty() {
                let p = assert_recv!(rx);
                assert_eq!(
                    <IdentifiedQoS as Into<QoS>>::into(p.identified_qos),
                    QoS::AtLeastOnce
                );

                let matching_msg = assert_ok!(
                    messages
                        .iter()
                        .enumerate()
                        .find(|(_, (t, m))| *t.as_ref() == p.topic && m == &msg)
                        .ok_or("Option is None")
                );

                messages.remove(matching_msg.0);
            }
        })
        .await;

        info!("{:?}", messages);
        assert!(messages.is_empty());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher1, publisher2);
}

#[tokio::test]
#[test_log::test]
async fn publish_recv_multiple_qos2() {
    let (topic_name1, topic_filter1) = unique_topic();
    let (topic_name2, topic_filter2) = unique_topic();
    let msg = "Standard Library? Where we're going, we don't need std.";

    let mut tx1 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx2 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher1 = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name1.clone(),
            qos: QoS::ExactlyOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx1, pub_options, msg.into());
        disconnect(&mut tx1, DEFAULT_DC_OPTIONS).await;
    };
    let publisher2 = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name2.clone(),
            qos: QoS::ExactlyOnce,
        };

        sleep(Duration::from_secs(2)).await;
        assert_published!(tx2, pub_options, msg.into());
        disconnect(&mut tx2, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::ExactlyOnce;

        assert_subscribe!(rx, options, topic_filter1.clone());
        assert_subscribe!(rx, options, topic_filter2.clone());

        let mut messages = vec![(topic_name1.clone(), msg), (topic_name2.clone(), msg)];

        let _ = timeout(Duration::from_secs(5), async {
            while !messages.is_empty() {
                let p = assert_recv!(rx);
                assert_eq!(
                    <IdentifiedQoS as Into<QoS>>::into(p.identified_qos),
                    QoS::ExactlyOnce
                );

                let matching_msg = assert_ok!(
                    messages
                        .iter()
                        .enumerate()
                        .find(|(_, (t, m))| *t.as_ref() == p.topic && m == &msg)
                        .ok_or("Option is None")
                );

                messages.remove(matching_msg.0);
            }
        })
        .await;

        info!("{:?}", messages);
        assert!(messages.is_empty());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher1, publisher2);
}

#[tokio::test]
#[test_log::test]
async fn unsub_no_recv() {
    let (topic_name1, topic_filter1) = unique_topic();
    let (topic_name2, topic_filter2) = unique_topic();
    let msg1 = "Smart Fridge: You are out of milk. And hope.";
    let msg2 = "Motion detected: It's the cat again, isn't it?";

    let mut tx1 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut tx2 =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher1 = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name1.clone(),
            qos: QoS::AtLeastOnce,
        };

        sleep(Duration::from_secs(5)).await;
        assert_published!(tx1, pub_options.clone(), msg1.into());
        sleep(Duration::from_secs(2)).await;
        assert_published!(tx1, pub_options, msg1.into());

        disconnect(&mut tx1, DEFAULT_DC_OPTIONS).await;
    };
    let publisher2 = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name2.clone(),
            qos: QoS::AtLeastOnce,
        };

        sleep(Duration::from_secs(6)).await;
        assert_published!(tx2, pub_options.clone(), msg2.into());
        sleep(Duration::from_secs(3)).await;
        assert_published!(tx2, pub_options, msg2.into());

        disconnect(&mut tx2, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;
        assert_subscribe!(rx, options, topic_filter1.clone());
        assert_subscribe!(rx, options, topic_filter2.clone());

        let m = assert_recv_excl!(rx, topic_name1);
        assert_eq!(&*m.message, msg1.as_bytes());

        let m = assert_recv_excl!(rx, topic_name2);
        assert_eq!(&*m.message, msg2.as_bytes());

        assert_unsubscribe!(rx, topic_filter2.clone());

        let m = assert_recv_excl!(rx, topic_name1);
        assert_eq!(&*m.message, msg1.as_bytes());

        assert_err!(
            timeout(Duration::from_secs(10), async {
                assert_recv!(rx);
            })
            .await,
            "Expected to receive nothing"
        );
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher1, publisher2);
}

#[tokio::test]
#[test_log::test]
async fn recv_min_sub_qos0() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "(╯°□°）╯︵ ┻━┻";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name.clone(),
            qos: QoS::ExactlyOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let options = DEFAULT_QOS0_SUB_OPTIONS;

        assert_subscribe!(rx, options, topic_filter.clone());
        let Publish {
            identified_qos,
            dup,
            retain,
            subscription_identifiers: _,
            topic: _,
            message,
        } = assert_recv_excl!(rx, topic_name);

        assert!(!dup);
        assert!(!retain);
        assert_eq!(
            <IdentifiedQoS as Into<QoS>>::into(identified_qos),
            QoS::AtMostOnce
        );
        assert_eq!(&*message, msg.as_bytes());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn recv_min_sub_qos1() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "¯\\_(ツ)_/¯";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name.clone(),
            qos: QoS::ExactlyOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::AtLeastOnce;

        assert_subscribe!(rx, options, topic_filter.clone());
        let Publish {
            identified_qos,
            dup,
            retain,
            subscription_identifiers: _,
            topic: _,
            message,
        } = assert_recv_excl!(rx, topic_name);

        assert!(!dup);
        assert!(!retain);
        assert_eq!(
            <IdentifiedQoS as Into<QoS>>::into(identified_qos),
            QoS::AtLeastOnce
        );
        assert_eq!(&*message, msg.as_bytes());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn recv_min_pub_qos0() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "ʕ•ᴥ•ʔ";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name.clone(),
            qos: QoS::AtMostOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::ExactlyOnce;

        assert_subscribe!(rx, options, topic_filter.clone());
        let Publish {
            identified_qos,
            dup,
            retain,
            subscription_identifiers: _,
            topic: _,
            message,
        } = assert_recv_excl!(rx, topic_name);

        assert!(!dup);
        assert!(!retain);
        assert_eq!(
            <IdentifiedQoS as Into<QoS>>::into(identified_qos),
            QoS::AtMostOnce
        );
        assert_eq!(&*message, msg.as_bytes());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[tokio::test]
#[test_log::test]
async fn recv_min_pub_qos1() {
    let (topic_name, topic_filter) = unique_topic();
    let msg = "┬─┬ ノ( ゜-゜ノ)";

    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);
    let mut rx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let publisher = async {
        let pub_options = PublicationOptions {
            retain: false,
            topic: topic_name.clone(),
            qos: QoS::AtLeastOnce,
        };

        sleep(Duration::from_secs(1)).await;
        assert_published!(tx, pub_options, msg.into());
        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        let mut options = DEFAULT_QOS0_SUB_OPTIONS;
        options.qos = QoS::ExactlyOnce;

        assert_subscribe!(rx, options, topic_filter.clone());
        let Publish {
            identified_qos,
            dup,
            retain,
            subscription_identifiers: _,
            topic: _,
            message,
        } = assert_recv_excl!(rx, topic_filter);

        assert!(!dup);
        assert!(!retain);
        assert_eq!(
            <IdentifiedQoS as Into<QoS>>::into(identified_qos),
            QoS::AtLeastOnce
        );
        assert_eq!(&*message, msg.as_bytes());
        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}
