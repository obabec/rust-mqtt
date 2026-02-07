use rust_mqtt::{
    Bytes,
    client::{MqttError, options::ConnectOptions},
    config::{KeepAlive, SessionExpiryInterval},
    types::MqttBinary,
};

use crate::common::{BROKER_ADDRESS, USERNAME, utils::connected_client};

const NO_CREDS_CONNECT_OPTIONS: ConnectOptions = ConnectOptions {
    clean_start: true,
    keep_alive: KeepAlive::Infinite,
    session_expiry_interval: SessionExpiryInterval::EndOnDisconnect,
    request_response_information: false,
    user_name: None,
    password: None,
    will: None,
};

#[tokio::test]
#[test_log::test]
async fn connect_no_creds() {
    let r = connected_client(BROKER_ADDRESS, &NO_CREDS_CONNECT_OPTIONS, None).await;

    assert!(r.is_err());
    let e = unsafe { r.unwrap_err_unchecked() };

    assert!(matches!(
        e,
        MqttError::Disconnect {
            reason: _,
            reason_string: _
        }
    ));
}

#[tokio::test]
#[test_log::test]
async fn connect_no_password() {
    let mut options = NO_CREDS_CONNECT_OPTIONS;
    options.user_name = Some(USERNAME);

    let r = connected_client(BROKER_ADDRESS, &options, None).await;

    assert!(r.is_err());
    let e = unsafe { r.unwrap_err_unchecked() };

    assert!(matches!(
        e,
        MqttError::Disconnect {
            reason: _,
            reason_string: _
        }
    ));
}

#[tokio::test]
#[test_log::test]
async fn connect_wrong_password() {
    let mut options = NO_CREDS_CONNECT_OPTIONS;
    options.user_name = Some(USERNAME);
    options.password = Some(MqttBinary::new(Bytes::Borrowed(b"wrong password")).unwrap());

    let r = connected_client(BROKER_ADDRESS, &options, None).await;

    assert!(r.is_err());
    let e = unsafe { r.unwrap_err_unchecked() };

    assert!(matches!(
        e,
        MqttError::Disconnect {
            reason: _,
            reason_string: _
        }
    ));
}
