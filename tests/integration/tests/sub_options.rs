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

use std::time::Duration;

use tokio::time::sleep;

use rust_mqtt::interface::{
    QualityOfService::QoS1,
    RetainHandling::{AlwaysSend, NeverSend, SendIfNotSubscribedBefore},
};

use crate::common::{assert::{assert_no_receive, assert_publish, assert_receive}, utils::{connected_client, disconnect, setup, subscribe}};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn subscribe_no_local_unset() {
    const TOPIC: &str = "test/no-local/false";
    const MSG: &str = "abc";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut client = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

    subscribe(&mut client, TOPIC, QoS1, Default::default(), false, false).await;

    sleep(Duration::from_millis(100)).await;

    let result = client
        .send_message(TOPIC, MSG.as_bytes(), QoS1, false)
        .await;

    if result.is_err() {
        // publish was received before puback, this is ok, there is nothing more we can check
    } else {
        assert_receive(&mut client, 5, TOPIC, MSG).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn subscribe_no_local_set() {
    const TOPIC: &str = "test/no-local/true";
    const MSG: &str = "abc";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut client = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

    subscribe(&mut client, TOPIC, QoS1, Default::default(), false, true).await;

    sleep(Duration::from_millis(100)).await;

    client
        .send_message(TOPIC, MSG.as_bytes(), QoS1, false)
        .await
        .expect("Error while publishing message");

    assert_no_receive(&mut client, 5).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn subscribe_retain_handling_default() {
    const TOPIC: &str = "test/retain-handling/default";
    const MSG: &str = "abc";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

    const RECEIVER_ID: &str = "RETAIN_HANDLING_DEFAULT_RECEIVER";
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut receiver =
        connected_client(&mut recv_buffer, &mut write_buffer, Some(RECEIVER_ID)).await;

    assert_publish(&mut publisher, 0, TOPIC, MSG, QoS1, true, false, true).await;

    sleep(Duration::from_secs(5)).await;

    subscribe(&mut receiver, TOPIC, QoS1, AlwaysSend, false, false).await;

    assert_receive(&mut receiver, 5, TOPIC, MSG).await;

    // Subscribing again should yield the retained message again
    subscribe(&mut receiver, TOPIC, QoS1, AlwaysSend, false, false).await;

    assert_receive(&mut receiver, 5, TOPIC, MSG).await;

    // Disconnecting and connecting again should yield the retained message again
    // This case makes only so much sense because currently clean session is true by default
    disconnect(&mut receiver).await;
    sleep(Duration::from_secs(1)).await;
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut receiver =
        connected_client(&mut recv_buffer, &mut write_buffer, Some(RECEIVER_ID)).await;

    subscribe(&mut receiver, TOPIC, QoS1, AlwaysSend, false, false).await;

    assert_receive(&mut receiver, 5, TOPIC, MSG).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn subscribe_retain_handling_never() {
    const TOPIC: &str = "test/retain-handling/never";
    const MSG: &str = "abc";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

    const RECEIVER_ID: &str = "RETAIN_HANDLING_NEVER_RECEIVER";
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut receiver =
        connected_client(&mut recv_buffer, &mut write_buffer, Some(RECEIVER_ID)).await;

    assert_publish(&mut publisher, 0, TOPIC, MSG, QoS1, true, false, true).await;

    sleep(Duration::from_secs(5)).await;

    subscribe(&mut receiver, TOPIC, QoS1, NeverSend, false, false).await;

    assert_no_receive(&mut receiver, 5).await;

    // Subscribing again should also not yield the retained message
    subscribe(&mut receiver, TOPIC, QoS1, NeverSend, false, false).await;

    assert_no_receive(&mut receiver, 5).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn subscribe_retain_handling_clean_only() {
    const TOPIC: &str = "test/retain-handling/clean-only";
    const MSG: &str = "abc";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

    const RECEIVER_ID: &str = "RETAIN_HANDLING_CLEAN_ONLY_RECEIVER";
    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    let mut receiver =
        connected_client(&mut recv_buffer, &mut write_buffer, Some(RECEIVER_ID)).await;

    assert_publish(&mut publisher, 0, TOPIC, MSG, QoS1, true, false, true).await;

    sleep(Duration::from_secs(5)).await;

    // Subscribing for the first time should yield the retained message
    subscribe(
        &mut receiver,
        TOPIC,
        QoS1,
        SendIfNotSubscribedBefore,
        false,
        false,
    )
    .await;

    assert_receive(&mut receiver, 5, TOPIC, MSG).await;

    // Subscribing again should not yield the retained message
    subscribe(
        &mut receiver,
        TOPIC,
        QoS1,
        SendIfNotSubscribedBefore,
        false,
        false,
    )
    .await;

    assert_no_receive(&mut receiver, 5).await;
}
