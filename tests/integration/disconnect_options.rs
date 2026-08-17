use std::{assert_eq, format, num::NonZero, panic};

use rust_mqtt::{
    client::{MqttError, options::DisconnectOptions},
    config::SessionExpiryInterval,
    types::{MqttString, MqttStringPair},
};

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

    let Err(e) = c.disconnect(&options).await else {
        panic!();
    };
    assert_eq!(e, MqttError::IllegalDisconnectSessionExpiryInterval);

    let options = DisconnectOptions::new()
        .session_expiry_interval(SessionExpiryInterval::Seconds(NonZero::new(1).unwrap()));

    let Err(e) = c.disconnect(&options).await else {
        panic!();
    };
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

#[ignore = "mosquitto has no configurable DISCONNECT maximum packet size"]
#[tokio::test]
#[test_log::test]
async fn server_maximum_packet_size_not_exceeded_by_disconnect_hive_only() {
    // fixed header, reason code, property length
    const OVERHEAD: u32 = 4 + 1 + 3;

    const SERVER_MAX_PACKET_SIZE: u32 = 2_000_000;

    const PROPERTY_SIZE: u32 = SERVER_MAX_PACKET_SIZE - OVERHEAD;

    let user_property = MqttStringPair::new(
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
    );

    const USER_PROPERTY_SIZE: u32 = (MqttString::MAX_LENGTH as u32 + 2) * 2 + 1;
    const REASON_STRING_SIZE: u32 = PROPERTY_SIZE - (USER_PROPERTIES * USER_PROPERTY_SIZE) - 3;

    const USER_PROPERTIES: u32 = PROPERTY_SIZE / USER_PROPERTY_SIZE;

    let user_properties: [_; USER_PROPERTIES as usize] =
        std::array::from_fn(|_| user_property.as_borrowed());

    // Exceed the limit
    let reason_string = MqttString::try_from(vec![b'a'; REASON_STRING_SIZE as usize]).unwrap();

    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    assert_ok!(
        c.disconnect(
            &DisconnectOptions::new()
                .user_properties(&user_properties)
                .reason_string(reason_string)
        )
        .await
    );
}

#[ignore = "mosquitto has no configurable DISCONNECT maximum packet size"]
#[tokio::test]
#[test_log::test]
async fn server_maximum_packet_size_exceeded_by_disconnect_hive_only() {
    // fixed header, reason code, property length
    const OVERHEAD: u32 = 4 + 1 + 3;

    const SERVER_MAX_PACKET_SIZE: u32 = 2_000_000;

    const PROPERTY_SIZE: u32 = SERVER_MAX_PACKET_SIZE - OVERHEAD;

    let user_property = MqttStringPair::new(
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
        MqttString::try_from(vec![b'a'; MqttString::MAX_LENGTH]).unwrap(),
    );

    const USER_PROPERTY_SIZE: u32 = (MqttString::MAX_LENGTH as u32 + 2) * 2 + 1;
    const REASON_STRING_SIZE: u32 = PROPERTY_SIZE - (USER_PROPERTIES * USER_PROPERTY_SIZE) - 3;

    const USER_PROPERTIES: u32 = PROPERTY_SIZE / USER_PROPERTY_SIZE;

    let user_properties: [_; USER_PROPERTIES as usize] =
        std::array::from_fn(|_| user_property.as_borrowed());
    let reason_string = MqttString::try_from(vec![b'a'; REASON_STRING_SIZE as usize + 1]).unwrap();

    let mut c =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let Err(e) = c
        .disconnect(
            &DisconnectOptions::new()
                .user_properties(&user_properties)
                .reason_string(reason_string),
        )
        .await
    else {
        panic!()
    };

    assert_eq!(e, MqttError::ServerMaximumPacketSizeExceeded);

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}
