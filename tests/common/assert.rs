use std::{str::from_utf8, time::Duration};

use log::{error, info};
use rust_mqtt::interface::{QualityOfService, ReasonCode};
use tokio::time::{sleep, timeout};
use tokio_test::{assert_err, assert_ok};

use crate::common::utils::{connect_core, TestClient};

pub async fn assert_wrong_credentials(
    recv_buffer: &mut [u8],
    write_buffer: &mut [u8],
    client_id: Option<&str>,
    username: &str,
    password: &str,
) {
    info!(
        "[WRONG_CREDS] Connecting with credentials {}:{}",
        username, password
    );

    let result = connect_core(recv_buffer, write_buffer, client_id, username, password).await;

    match result {
        Ok(_) => panic!("[WRONG CREDS] Connection succeeded against expectations"),
        Err(ReasonCode::NotAuthorized) => {
            info!("[WRONG CREDS] Connection failed due to wrong creds")
        }
        Err(e) => panic!("[WRONG CREDS] Connection failed unexpectedly: {}", e),
    }
}

pub async fn assert_publish(
    client: &mut TestClient<'_>,
    wait: u64,
    topic: &str,
    msg: &str,
    qos: QualityOfService,
    retain: bool,
    should_err: bool,
    can_err: bool,
) {
    sleep(Duration::from_secs(wait)).await;

    info!("[PUBLISH] Publishing {} to {}", msg, topic);

    let result = client
        .send_message(topic, msg.as_bytes(), qos, retain)
        .await;

    if should_err {
        let e = assert_err!(
            result,
            "[PUBLISH] Publishing {} to {} succeeded against expectations",
            msg,
            topic
        );
        info!(
            "[PUBLISH] Publishing {} to {} failed successfully: {}",
            msg, topic, e
        );
    } else if can_err {
        match result {
            Ok(_) => info!("[PUBLISH] Publishing {} to {} succeeded", msg, topic),
            Err(e) => info!(
                "[PUBLISH] Publishing {} to {} failed ({}), but is allowed to error",
                msg, topic, e
            ),
        }
    } else {
        assert_ok!(result, "[PUBLISH] Publishing {} to {} failed", msg, topic);
    }
}

pub async fn assert_receive(client: &mut TestClient<'_>, within: u64, topic: &str, msg: &str) {
    let duration = Duration::from_secs(within);

    info!(
        "[RECEIVE] Awaiting receival of {} on topic {} in the next {}s",
        msg, topic, within
    );
    let result = timeout(duration, client.receive_message()).await;

    let result = assert_ok!(
        result,
        "[RECEIVE] Receival of {} on topic {} timed out after {}s",
        msg,
        topic,
        within
    );

    let (recv_topic, recv_msg) = assert_ok!(
        result,
        "[RECEIVE] Receival of {} on topic {} failed",
        msg,
        topic
    );

    assert_eq!(topic, recv_topic);
    assert_eq!(msg.as_bytes(), recv_msg);
}

pub async fn assert_no_receive(client: &mut TestClient<'_>, within: u64) {
    let duration = Duration::from_secs(within);

    info!("[NORECEIVE] Expecting no receival in the next {}s", within);

    let result = timeout(duration, client.receive_message()).await;

    match result {
        Ok(Ok((recv_topic, recv_msg))) => panic!(
            "[NORECEIVE] Received message {:?} on topic {}",
            recv_msg, recv_topic
        ),
        Ok(Err(e)) => panic!("[NORECEIVE] Received error: {}", e),
        Err(_) => info!("[NORECEIVE] Received no message in the last {}s", within),
    }
}

pub async fn assert_receive_multiple(
    client: &mut TestClient<'_>,
    within: u64,
    msgs: Vec<(&str, &str)>,
) {
    let duration = Duration::from_secs(within);

    let mut msgs = msgs;

    info!(
        "[RECV_MUL] Expecting messages {:?} in the next {}s",
        msgs, within
    );

    let result = timeout(duration, async {
        loop {
            let message = client.receive_message().await;
            assert_ok!(message);
            let (topic, msg) = message.unwrap();
            let msg = from_utf8(msg);
            assert_ok!(msg);
            let msg = msg.unwrap();

            let matching_msg = msgs
                .iter()
                .enumerate()
                .find(|(_, (t, m))| *t == topic && m == &msg);
            if matching_msg.is_none() {
                panic!("[RECV_MUL] Received unexpected message: {}:{}, Still receivable messages: {:?}", topic, msg, msgs);
            } else {
                info!("[RECV_MUL] Received message: {}:{}", topic, msg);
            }

            msgs.remove(matching_msg.unwrap().0);
        }
    })
    .await;

    if !msgs.is_empty() {
        panic!(
            "[RECV_MUL] Didn't receive all messages in time. Missing messages: {:?}",
            msgs
        );
    }
}
