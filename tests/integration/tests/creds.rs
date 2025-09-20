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

use tokio::task;

use crate::common::{
    assert::{assert_publish, assert_receive, assert_wrong_credentials},
    utils::{connected_client, disconnect, setup, subscribe},
};

use rust_mqtt::interface::{
    QualityOfService::{QoS0, QoS1},
    RetainHandling::AlwaysSend,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn publish_recv_wrong_cred() {
    const USERNAME: &str = "test";
    const WRONG_PASSWORD: &str = "xyz";

    const TOPIC: &str = "test/recv/wrong";
    const MSG: &str = "testMessage";

    setup();

    let mut recv_buffer = [0; 100];
    let mut write_buffer = [0; 100];
    assert_wrong_credentials(&mut recv_buffer, &mut write_buffer, None, USERNAME, WRONG_PASSWORD).await;

    let t1 = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut publisher = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        assert_publish(&mut publisher, 5, TOPIC, MSG, QoS1, false, false, false).await;
        disconnect(&mut publisher).await;
    });

    let t2 = task::spawn(async {
        let mut recv_buffer = [0; 100];
        let mut write_buffer = [0; 100];
        let mut receiver = connected_client(&mut recv_buffer, &mut write_buffer, None).await;

        subscribe(&mut receiver, TOPIC, QoS0, AlwaysSend, false, false).await;
        assert_receive(&mut receiver, 10, TOPIC, MSG).await;
        disconnect(&mut receiver).await;
    });
    t1.await.unwrap();
    t2.await.unwrap();
}
