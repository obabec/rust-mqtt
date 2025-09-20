/*
 * MIT License
 *
 * Copyright (c) [2022] [Ondrej Babec <ond.babec@gmail.com>]
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use rust_mqtt::interface::{
    QualityOfService::{QoS0, QoS1},
    RetainHandling::AlwaysSend,
};
use tokio::task;

use crate::common::{
    assert::{assert_no_receive, assert_publish, assert_receive, assert_receive_multiple},
    utils::{connected_client, disconnect, setup, subscribe, unsubscribe},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn publish_recv() {
    const TOPIC: &str = "test/recv/simple";
    const MSG: &str = "testMessage";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut receiver = connected_client(&mut recv_buffer, &mut write_buffer, None).await;
    subscribe(&mut receiver, TOPIC, QoS0, AlwaysSend, false, false).await;

    let t = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        assert_publish(&mut publisher, 5, TOPIC, MSG, QoS0, false, false, false).await;
        disconnect(&mut publisher).await;
    });

    assert_receive(&mut receiver, 10, TOPIC, MSG).await;
    disconnect(&mut receiver).await;
    t.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn publish_recv_qos() {
    const TOPIC: &str = "test/recv/qos";
    const MSG: &str = "testMessage";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut receiver = connected_client(&mut recv_buffer, &mut write_buffer, None).await;
    subscribe(&mut receiver, TOPIC, QoS1, AlwaysSend, false, false).await;

    let t = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        assert_publish(&mut publisher, 5, TOPIC, MSG, QoS1, false, false, false).await;
        disconnect(&mut publisher).await;
    });

    assert_receive(&mut receiver, 10, TOPIC, MSG).await;
    disconnect(&mut receiver).await;
    t.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn publish_recv_multiple() {
    const TOPIC_1: &str = "test/topic1";
    const TOPIC_2: &str = "test/topic2";
    const MSG: &str = "testMessage";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut receiver = connected_client(&mut recv_buffer, &mut write_buffer, None).await;
    subscribe(&mut receiver, TOPIC_1, QoS0, AlwaysSend, false, false).await;
    subscribe(&mut receiver, TOPIC_2, QoS0, AlwaysSend, false, false).await;

    let t1 = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        assert_publish(&mut publisher, 5, TOPIC_1, MSG, QoS0, false, false, false).await;
        disconnect(&mut publisher).await;
    });

    let t2 = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        assert_publish(&mut publisher, 10, TOPIC_2, MSG, QoS0, false, false, false).await;
        disconnect(&mut publisher).await;
    });

    assert_receive_multiple(&mut receiver, 15, vec![(TOPIC_1, MSG), (TOPIC_2, MSG)]).await;
    disconnect(&mut receiver).await;
    t1.await.unwrap();
    t2.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn publish_recv_multiple_qos() {
    const TOPIC_1: &str = "test/topic3";
    const TOPIC_2: &str = "test/topic4";
    const MSG: &str = "testMessage";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut receiver = connected_client(&mut recv_buffer, &mut write_buffer, None).await;
    subscribe(&mut receiver, TOPIC_1, QoS1, AlwaysSend, false, false).await;
    subscribe(&mut receiver, TOPIC_2, QoS1, AlwaysSend, false, false).await;

    let t1 = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        assert_publish(&mut publisher, 5, TOPIC_1, MSG, QoS1, false, false, false).await;
        disconnect(&mut publisher).await;
    });

    let t2 = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        assert_publish(&mut publisher, 10, TOPIC_2, MSG, QoS1, false, false, false).await;
        disconnect(&mut publisher).await;
    });

    assert_receive_multiple(&mut receiver, 15, vec![(TOPIC_1, MSG), (TOPIC_2, MSG)]).await;
    disconnect(&mut receiver).await;
    t1.await.unwrap();
    t2.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn sub_unsub() {
    const TOPIC_1: &str = "test/unsub/topic1";
    const TOPIC_2: &str = "test/unsub/topic2";
    const MSG_1: &str = "First topic message";
    const MSG_2: &str = "Second topic message";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut receiver = connected_client(&mut recv_buffer, &mut write_buffer, None).await;
    subscribe(&mut receiver, TOPIC_1, QoS1, AlwaysSend, false, false).await;
    subscribe(&mut receiver, TOPIC_2, QoS1, AlwaysSend, false, false).await;

    let t1 = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        assert_publish(&mut publisher, 5, TOPIC_1, MSG_1, QoS1, false, false, false).await;
        assert_publish(&mut publisher, 2, TOPIC_1, MSG_1, QoS1, false, false, false).await;
    });
    let t2 = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        assert_publish(&mut publisher, 6, TOPIC_2, MSG_2, QoS1, false, false, false).await;
        assert_publish(&mut publisher, 3, TOPIC_2, MSG_2, QoS1, false, false, true).await;
    });

    assert_receive(&mut receiver, 10, TOPIC_1, MSG_1).await;
    assert_receive(&mut receiver, 5, TOPIC_2, MSG_2).await;
    unsubscribe(&mut receiver, TOPIC_2).await;
    assert_receive(&mut receiver, 5, TOPIC_1, MSG_1).await;
    assert_no_receive(&mut receiver, 10).await;

    t1.await.unwrap();
    t2.await.unwrap();
}
