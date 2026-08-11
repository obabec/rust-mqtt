use std::{assert_eq, format};

use rust_mqtt::{
    client::{MqttError, options::DisconnectOptions},
    config::SessionExpiryInterval,
    types::{MqttString, MqttStringPair},
};
use tokio_test::assert_err;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::assert_ok,
    utils::{connected_client, disconnect},
};

#[tokio::test]
#[test_log::test]
async fn connect_session_expiry_zero_disconnect_session_expiry_non_zero() {
    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let options = DisconnectOptions::new().session_expiry_interval(SessionExpiryInterval::NeverEnd);

    let e = assert_err!(c.disconnect(&options).await);
    assert_eq!(e, MqttError::IllegalDisconnectSessionExpiryInterval);

    let options =
        DisconnectOptions::new().session_expiry_interval(SessionExpiryInterval::Seconds(1));

    let e = assert_err!(c.disconnect(&options).await);
    assert_eq!(e, MqttError::IllegalDisconnectSessionExpiryInterval);

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[tokio::test]
#[test_log::test]
async fn reason_string() {
    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    assert_ok!(
        c.disconnect(
            &DisconnectOptions::new().reason_string(
                MqttString::from_str(
                    "Disconnected: The TCP handshake was too firm and scared the client."
                )
                .unwrap()
            )
        )
        .await
    );
}

#[tokio::test]
#[test_log::test]
async fn user_properties() {
    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let user_properties: [_; 16] = std::array::from_fn(|i| {
        MqttStringPair::new(
            MqttString::try_from(format!("key_{i}")).unwrap(),
            MqttString::try_from(format!("value_{i}")).unwrap(),
        )
    });

    assert_ok!(
        c.disconnect(&DisconnectOptions::new().user_properties(&user_properties[..]))
            .await
    );
}

#[should_panic]
#[tokio::test]
#[test_log::test]
async fn too_many_user_properties() {
    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let user_properties: [_; 17] = std::array::from_fn(|i| {
        MqttStringPair::new(
            MqttString::try_from(format!("key_{i}")).unwrap(),
            MqttString::try_from(format!("value_{i}")).unwrap(),
        )
    });

    let _ = c
        .disconnect(&DisconnectOptions::new().user_properties(&user_properties[..]))
        .await;
}
