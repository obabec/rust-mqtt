use std::time::Duration;

use rust_mqtt::{client::Client, config::SessionExpiryInterval, types::MqttString};
use tokio::time::sleep;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, NO_SESSION_CONNECT_OPTIONS,
    assert::assert_ok,
    fmt::warn_inspect,
    utils::{ALLOC, connected_client, disconnect, tcp_connection},
};

#[tokio::test]
#[test_log::test]
async fn session_continue_regular_disconnect() {
    let id = MqttString::new("SESSION_CONTINUE_REGULAR_DISCONNECT_CLIENT".into()).unwrap();

    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.session_expiry_interval = SessionExpiryInterval::NeverEnd;

    let mut c = assert_ok!(
        connected_client(BROKER_ADDRESS, &connect_options, Some(id.as_borrowed())).await
    );

    sleep(Duration::from_secs(5)).await;

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;

    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(3);
    connect_options.clean_start = false;

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);

    let info = assert_ok!(warn_inspect!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await,
        "Client::connect() failed"
    ));

    assert!(info.session_present);

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;

    // Wait just long enough so the session hasn't expired yet.
    sleep(Duration::from_secs(2)).await;

    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.clean_start = false;

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);

    let info = assert_ok!(warn_inspect!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await,
        "Client::connect() failed"
    ));

    assert!(info.session_present);

    let mut dc_options = DEFAULT_DC_OPTIONS.clone();
    dc_options.session_expiry_interval = Some(SessionExpiryInterval::EndOnDisconnect);
    disconnect(&mut c, &dc_options).await;
}

#[tokio::test]
#[test_log::test]
async fn session_continue_connection_dropped() {
    let id = MqttString::new("SESSION_CONTINUE_CONNECTION_DROPPED_CLIENT".into()).unwrap();

    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.session_expiry_interval = SessionExpiryInterval::NeverEnd;

    let c = assert_ok!(
        connected_client(BROKER_ADDRESS, &connect_options, Some(id.as_borrowed())).await
    );

    sleep(Duration::from_secs(5)).await;

    let session = c.session().clone();
    drop(c);

    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(3);
    connect_options.clean_start = false;

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
    let mut c: Client<'_, _, _, 1, 1, 1, 1> = Client::with_session(session, ALLOC.get());
    let info = assert_ok!(warn_inspect!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await,
        "Client::connect() failed"
    ));
    assert!(info.session_present);

    let session = c.session().clone();
    drop(c);

    // Wait just long enough so the session hasn't expired yet.
    sleep(Duration::from_secs(2)).await;

    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.clean_start = false;

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
    let mut c = Client::with_session(session, ALLOC.get());
    let info = assert_ok!(warn_inspect!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await,
        "Client::connect() failed"
    ));
    assert!(info.session_present);

    let mut dc_options = DEFAULT_DC_OPTIONS.clone();
    dc_options.session_expiry_interval = Some(SessionExpiryInterval::EndOnDisconnect);
    disconnect(&mut c, &dc_options).await;
}

#[tokio::test]
#[test_log::test]
async fn session_discontinued_clean_start() {
    let id = MqttString::new("SESSION_DISCONTINUED_CLEAN_START_CLIENT".into()).unwrap();

    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.session_expiry_interval = SessionExpiryInterval::NeverEnd;

    let mut c = assert_ok!(
        connected_client(BROKER_ADDRESS, &connect_options, Some(id.as_borrowed())).await
    );
    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
    let info = assert_ok!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await
    );
    assert!(!info.session_present);

    let session = c.session().clone();
    drop(c);

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
    let mut c: Client<'_, _, _, 1, 1, 1, 1> = Client::with_session(session, ALLOC.get());
    let info = assert_ok!(warn_inspect!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await,
        "Client::connect() failed"
    ));
    assert!(!info.session_present);

    let mut dc_options = DEFAULT_DC_OPTIONS.clone();
    dc_options.session_expiry_interval = Some(SessionExpiryInterval::EndOnDisconnect);
    disconnect(&mut c, &dc_options).await;
}

#[tokio::test]
#[test_log::test]
async fn session_expired() {
    let id = MqttString::new("SESSION_EXPIRED_CLIENT".into()).unwrap();

    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(3);

    let mut c = assert_ok!(
        connected_client(BROKER_ADDRESS, &connect_options, Some(id.as_borrowed())).await
    );
    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;

    // Sleep long enough for the session expiry interval to expire
    sleep(Duration::from_secs(5)).await;

    // Try to continue the previous session
    connect_options.clean_start = false;

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
    let info = assert_ok!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await
    );
    assert!(!info.session_present);

    let session = c.session().clone();
    drop(c);

    // Sleep long enough for the session expiry interval to expire
    sleep(Duration::from_secs(5)).await;

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
    let mut c: Client<'_, _, _, 1, 1, 1, 1> = Client::with_session(session, ALLOC.get());
    let info = assert_ok!(warn_inspect!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await,
        "Client::connect() failed"
    ));
    assert!(!info.session_present);

    let mut dc_options = DEFAULT_DC_OPTIONS.clone();
    dc_options.session_expiry_interval = Some(SessionExpiryInterval::EndOnDisconnect);
    disconnect(&mut c, &dc_options).await;
}

#[tokio::test]
#[test_log::test]
async fn session_shortened_and_expired() {
    let id = MqttString::new("SESSION_SHORTENED_AND_EXPIRED_CLIENT".into()).unwrap();

    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.session_expiry_interval = SessionExpiryInterval::NeverEnd;

    let mut c = assert_ok!(
        connected_client(BROKER_ADDRESS, &connect_options, Some(id.as_borrowed())).await
    );

    let mut dc_options = DEFAULT_DC_OPTIONS.clone();
    dc_options.session_expiry_interval = Some(SessionExpiryInterval::Seconds(3));
    disconnect(&mut c, &dc_options).await;

    // Sleep long enough for the session expiry interval set in disconnect to expire
    sleep(Duration::from_secs(5)).await;

    // Try to continue the previous session
    let mut connect_options = NO_SESSION_CONNECT_OPTIONS.clone();
    connect_options.session_expiry_interval = SessionExpiryInterval::Seconds(10);
    connect_options.clean_start = false;

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
    let info = assert_ok!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await
    );
    assert!(!info.session_present);

    let mut dc_options = DEFAULT_DC_OPTIONS.clone();
    dc_options.session_expiry_interval = Some(SessionExpiryInterval::EndOnDisconnect);
    disconnect(&mut c, &dc_options).await;

    // Session ends right away, try to continue it directly

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);
    connect_options.session_expiry_interval = SessionExpiryInterval::EndOnDisconnect;
    let info = assert_ok!(
        c.connect(tcp, &connect_options, Some(id.as_borrowed()))
            .await
    );
    assert!(!info.session_present);

    let mut dc_options = DEFAULT_DC_OPTIONS.clone();
    dc_options.session_expiry_interval = Some(SessionExpiryInterval::EndOnDisconnect);
    disconnect(&mut c, &dc_options).await;
}
