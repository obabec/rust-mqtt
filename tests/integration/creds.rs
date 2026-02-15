use rust_mqtt::{
    Bytes,
    client::{MqttError, options::ConnectOptions},
    types::MqttBinary,
};

use crate::common::{BROKER_ADDRESS, USERNAME, utils::connected_client};

const NO_CREDS_CONNECT_OPTIONS: ConnectOptions = ConnectOptions::new().clean_start();

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
    let options = NO_CREDS_CONNECT_OPTIONS.user_name(USERNAME);

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
    let options = NO_CREDS_CONNECT_OPTIONS
        .user_name(USERNAME)
        .password(MqttBinary::from_bytes(Bytes::Borrowed(b"wrong password")).unwrap());

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
