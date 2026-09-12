use std::{
    assert_eq, matches,
    net::SocketAddr,
    panic,
    str::from_utf8,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use log::{debug, error, warn};
use pbkdf2::pbkdf2_hmac;
use rand::{Rng, distributions::Alphanumeric};
use rust_mqtt::{
    auth::{AuthMechanism, AuthOptions},
    client::{
        Client, MqttError,
        event::{Auth, Event},
        options::{
            ConnectOptions, PublicationOptions, ReAuthOptions, SubscriptionOptions, TopicReference,
            UnsubscriptionOptions,
        },
    },
    types::{MqttBinary, MqttString, ReasonCode},
};
use sha2::{Digest, Sha256};
use tokio::{
    join,
    time::{sleep, timeout},
};
use tokio_test::assert_err;

use crate::common::{
    BROKER_ADDRESS, DEFAULT_DC_OPTIONS, NO_SESSION_CONNECT_OPTIONS, PASSWORD, TestClient, USERNAME,
    assert::{assert_ok, assert_published, assert_subscribe},
    fmt::warn_inspect,
    utils::{ALLOC, connected_client, disconnect, tcp_connection, unique_topic},
};

const NO_CREDS_CONNECT_OPTIONS: ConnectOptions = ConnectOptions::new().clean_start();
const SCRAM_METHOD: MqttString = MqttString::from_str_unchecked("SCRAM-SHA-256");

fn scram() -> (ScramSha256, MqttBinary<'static>) {
    ScramSha256::new(USERNAME.as_str(), from_utf8(PASSWORD.as_bytes()).unwrap())
}

async fn connected_enhanced_auth_client(
    broker: SocketAddr,
    client_identifier: Option<MqttString<'_>>,
) -> Result<TestClient<'static>, MqttError<'static, 16, ScramError>> {
    let mut client = Client::new(ALLOC.get());

    let tcp = tcp_connection(broker)
        .await
        .map_err(MqttError::into_fallible)?;

    let (mut scram, first) = scram();

    warn_inspect!(
        client
            .connect_enhanced(
                tcp,
                &NO_CREDS_CONNECT_OPTIONS.authentication_data(first),
                client_identifier,
                SCRAM_METHOD,
                &mut scram
            )
            .await,
        "Client::connect() failed"
    )
    .map(|_| client)
}

#[ignore = "enhanced authentication is only supported out of the box by emqx"]
#[tokio::test]
#[test_log::test]
async fn connect_enhanced_emqx_only() {
    let mut c = Client::new(ALLOC.get());

    let (mut scram, first) = scram();

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);

    assert_ok!(
        c.connect_enhanced(
            tcp,
            &NO_CREDS_CONNECT_OPTIONS.clone().authentication_data(first),
            None,
            SCRAM_METHOD,
            &mut scram,
        )
        .await
    );

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[ignore = "enhanced authentication is only supported out of the box by emqx"]
#[tokio::test]
#[test_log::test]
async fn reauthenticate_emqx_only() {
    let mut c = assert_ok!(connected_enhanced_auth_client(BROKER_ADDRESS, None).await);

    let (mut scram, first) = scram();

    assert_ok!(
        timeout(Duration::from_secs(5), async {
            assert_ok!(
                c.reauthenticate(&ReAuthOptions::new().authentication_data(first))
                    .await
            );

            loop {
                match assert_ok!(c.poll().await) {
                    Event::Auth(auth) if auth.reason_code == ReasonCode::Success => {
                        assert_ok!(scram.success(&auth));
                        break;
                    }

                    Event::Auth(auth) => {
                        let options = assert_ok!(scram.kontinue(&auth));
                        let options = ReAuthOptions {
                            authentication_data: options.authentication_data,
                            ..Default::default()
                        };
                        assert_ok!(c.reauthenticate(&options).await)
                    }
                    _ => warn!("received unexpected event"),
                }
            }
        })
        .await
    );

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[ignore = "enhanced authentication is only supported out of the box by emqx"]
#[tokio::test]
#[test_log::test]
async fn reauthenticate_with_publish_traffic_emqx_only() {
    let mut rx = assert_ok!(connected_enhanced_auth_client(BROKER_ADDRESS, None).await);
    let mut tx =
        assert_ok!(connected_client(BROKER_ADDRESS, NO_SESSION_CONNECT_OPTIONS, None).await);

    let (in_topic_name, in_topic_filter) = unique_topic();
    let (out_topic_name, _) = unique_topic();

    let reauthenticated = AtomicBool::new(false);

    let msg = "Modern security: I prove I am me by unlocking a phone that proves I am me to approve a code that proves I am me.";

    let publisher = async {
        while !reauthenticated.load(Ordering::Relaxed) {
            let pub_options =
                PublicationOptions::new(TopicReference::Name(in_topic_name.as_borrowed())).retain();

            sleep(Duration::from_millis(5)).await;
            assert_published!(tx, pub_options, msg.into());
        }

        disconnect(&mut tx, DEFAULT_DC_OPTIONS).await;
    };

    let receiver = async {
        assert_subscribe!(
            rx,
            &SubscriptionOptions::new().at_least_once(),
            in_topic_filter
        );

        let (mut scram, first) = scram();

        assert_ok!(
            rx.reauthenticate(&ReAuthOptions::new().authentication_data(first))
                .await
        );

        loop {
            match rx
                .publish(
                    &PublicationOptions::new(TopicReference::Name(out_topic_name.as_borrowed()))
                        .exactly_once(),
                    msg.into(),
                )
                .await
            {
                Ok(_) => {}
                Err(e) => assert_eq!(e, MqttError::SendQuotaExceeded),
            }

            match assert_ok!(rx.poll().await) {
                Event::Auth(auth) if auth.reason_code == ReasonCode::Success => {
                    sleep(Duration::from_millis(100)).await;
                    assert_ok!(scram.success(&auth));
                    break;
                }
                Event::Auth(auth) => {
                    sleep(Duration::from_millis(100)).await;
                    let options = assert_ok!(scram.kontinue(&auth));
                    let options = ReAuthOptions {
                        authentication_data: options.authentication_data,
                        ..Default::default()
                    };
                    assert_ok!(rx.reauthenticate(&options).await)
                }
                Event::Publish(_) => {}
                Event::PublishReceived(_) | Event::PublishComplete(_) => {}
                e => warn!("received unexpected event: {:?}", e),
            }
        }

        reauthenticated.store(true, Ordering::Relaxed);

        disconnect(&mut rx, DEFAULT_DC_OPTIONS).await;
    };

    join!(receiver, publisher);
}

#[ignore = "enhanced authentication is only supported out of the box by emqx"]
#[tokio::test]
#[test_log::test]
async fn reauthenticate_with_sub_unsub_traffic_emqx_only() {
    let mut c = assert_ok!(connected_enhanced_auth_client(BROKER_ADDRESS, None).await);

    let (mut scram, first) = scram();

    assert_ok!(
        c.reauthenticate(&ReAuthOptions::new().authentication_data(first))
            .await
    );

    loop {
        match c
            .subscribe(unique_topic().1, &SubscriptionOptions::new())
            .await
        {
            Ok(_) => {}
            Err(e) => assert_eq!(e, MqttError::SessionBuffer),
        }
        match c
            .unsubscribe(unique_topic().1, &UnsubscriptionOptions::new())
            .await
        {
            Ok(_) => {}
            Err(e) => assert_eq!(e, MqttError::SessionBuffer),
        }

        match assert_ok!(c.poll().await) {
            Event::Auth(auth) if auth.reason_code == ReasonCode::Success => {
                sleep(Duration::from_millis(100)).await;
                assert_ok!(scram.success(&auth));
                break;
            }
            Event::Auth(auth) => {
                sleep(Duration::from_millis(100)).await;
                let options = assert_ok!(scram.kontinue(&auth));
                let options = ReAuthOptions {
                    authentication_data: options.authentication_data,
                    ..Default::default()
                };
                assert_ok!(c.reauthenticate(&options).await)
            }
            Event::Suback(_) | Event::Unsuback(_) => {}
            e => warn!("received unexpected event: {:?}", e),
        }
    }

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[ignore = "enhanced authentication is only supported out of the box by emqx"]
#[tokio::test]
#[test_log::test]
async fn reauthenticate_with_ping_traffic_emqx_only() {
    let mut c = assert_ok!(connected_enhanced_auth_client(BROKER_ADDRESS, None).await);

    let (mut scram, first) = scram();

    assert_ok!(
        c.reauthenticate(&ReAuthOptions::new().authentication_data(first))
            .await
    );

    loop {
        assert_ok!(c.ping().await);

        match assert_ok!(c.poll().await) {
            Event::Auth(auth) if auth.reason_code == ReasonCode::Success => {
                sleep(Duration::from_millis(100)).await;
                assert_ok!(scram.success(&auth));
                break;
            }
            Event::Auth(auth) => {
                sleep(Duration::from_millis(100)).await;
                let options = assert_ok!(scram.kontinue(&auth));
                let options = ReAuthOptions {
                    authentication_data: options.authentication_data,
                    ..Default::default()
                };
                assert_ok!(c.reauthenticate(&options).await)
            }
            Event::Pingresp => {}
            e => warn!("received unexpected event: {:?}", e),
        }
    }

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[ignore = "enhanced authentication is only supported out of the box by emqx"]
#[tokio::test]
#[test_log::test]
async fn authenticate_handshake_violation_emqx_only() {
    let mut client: TestClient = Client::new(ALLOC.get());

    let tcp = assert_ok!(tcp_connection(BROKER_ADDRESS).await);

    let (mut scram, first) = scram();

    assert_err!(
        client
            .connect_enhanced(
                tcp,
                &NO_CREDS_CONNECT_OPTIONS.authentication_data(first),
                None,
                MqttString::from_str("SCRAM-SHA-512").unwrap(),
                &mut scram
            )
            .await
    );
}

#[ignore = "enhanced authentication is only supported out of the box by emqx"]
#[tokio::test]
#[test_log::test]
async fn reauthenticate_handshake_violation_prevented_emqx_only() {
    let mut c = assert_ok!(connected_enhanced_auth_client(BROKER_ADDRESS, None).await);

    let (_, first) = scram();

    assert_ok!(
        c.reauthenticate(&ReAuthOptions::new().authentication_data(first))
            .await
    );

    let e = assert_err!(c.reauthenticate(&ReAuthOptions::new()).await);
    assert_eq!(e, MqttError::ReauthenticationHandshakeStateMismatched);

    disconnect(&mut c, DEFAULT_DC_OPTIONS).await;
}

#[ignore = "enhanced authentication is only supported out of the box by emqx"]
#[tokio::test]
#[test_log::test]
async fn reauthenticate_handshake_violation_emqx_only() {
    let mut c = assert_ok!(connected_enhanced_auth_client(BROKER_ADDRESS, None).await);

    assert_ok!(
        c.reauthenticate(
            &ReAuthOptions::new()
                .authentication_data(MqttBinary::from_slice("gibberish".as_bytes()).unwrap())
        )
        .await
    );

    let e = assert_err!(c.poll().await);
    assert!(matches!(e, MqttError::Disconnect { .. }));
}

#[derive(Debug)]
struct ScramError;

#[derive(Debug, Clone)]
enum State {
    AwaitServerFirst {
        client_nonce: String,
        client_first_bare: String,
    },
    AwaitServerFinal {
        server_signature: Vec<u8>,
    },
    Complete,
}

#[derive(Debug, Clone)]
struct ScramSha256 {
    password: String,
    state: State,
}

impl ScramSha256 {
    pub fn new(username: &str, password: impl Into<String>) -> (Self, MqttBinary<'static>) {
        let username = username.replace('=', "=2D").replace(',', "=2C"); // RFC 5802 escaping
        let client_nonce = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(24)
            .map(char::from)
            .collect();
        let client_first_bare = format!("n={},r={}", username, client_nonce);
        let client_first = format!("n,,{}", client_first_bare);

        (
            Self {
                password: password.into(),
                state: State::AwaitServerFirst {
                    client_nonce,
                    client_first_bare,
                },
            },
            MqttBinary::try_from(client_first.into_bytes()).unwrap(),
        )
    }
}

impl<const MAX_USER_PROPERTIES: usize> AuthMechanism<MAX_USER_PROPERTIES> for ScramSha256 {
    type Error = ScramError;

    fn kontinue(
        &mut self,
        auth: &Auth<'_, MAX_USER_PROPERTIES>,
    ) -> Result<AuthOptions<'_, MAX_USER_PROPERTIES>, (Self::Error, Option<ReasonCode>)> {
        let State::AwaitServerFirst {
            client_nonce,
            client_first_bare,
        } = &self.state
        else {
            return Err((ScramError, Some(ReasonCode::ProtocolError)));
        };

        let server_first = auth
            .authentication_data
            .as_ref()
            .and_then(|d| from_utf8(d.as_bytes()).ok())
            .ok_or((ScramError, Some(ReasonCode::ProtocolError)))?;

        let full_nonce =
            get_attr(server_first, "r=").map_err(|e| (e, Some(ReasonCode::ProtocolError)))?;
        let salt = STANDARD
            .decode(get_attr(server_first, "s=").map_err(|e| (e, Some(ReasonCode::ProtocolError)))?)
            .map_err(|_| (ScramError, Some(ReasonCode::ProtocolError)))?;
        let iter = get_attr(server_first, "i=")
            .map_err(|e| (e, Some(ReasonCode::ProtocolError)))?
            .parse::<u32>()
            .map_err(|_| (ScramError, Some(ReasonCode::ProtocolError)))?;

        debug!(
            "full_nonce={}, salt={:?}, iterations={}",
            full_nonce, salt, iter
        );

        if !full_nonce.starts_with(client_nonce) {
            error!(
                "server nonce (={}) does not start with client nonce (={})",
                full_nonce, client_nonce
            );

            return Err((ScramError, Some(ReasonCode::ProtocolError)));
        }

        let mut salted_password = [0u8; 32];
        pbkdf2_hmac::<Sha256>(self.password.as_bytes(), &salt, iter, &mut salted_password);

        let client_key = hmac(&salted_password, b"Client Key");
        let stored_key = Sha256::digest(client_key);
        let server_key = hmac(&salted_password, b"Server Key");

        let client_final_no_proof = format!("c=biws,r={}", full_nonce);
        let auth_msg = format!(
            "{},{},{}",
            client_first_bare, server_first, client_final_no_proof
        );

        let client_sig = hmac(&stored_key, auth_msg.as_bytes());
        let client_proof: Vec<u8> = client_key
            .iter()
            .zip(client_sig.iter())
            .map(|(a, b)| a ^ b)
            .collect();

        self.state = State::AwaitServerFinal {
            server_signature: hmac(&server_key, auth_msg.as_bytes()).to_vec(),
        };

        let response = format!(
            "{},p={}",
            client_final_no_proof,
            STANDARD.encode(client_proof)
        );

        debug!("client_final={}", response);

        Ok(AuthOptions {
            authentication_data: Some(MqttBinary::try_from(response.into_bytes()).unwrap()),
            ..Default::default()
        })
    }

    fn success(
        &mut self,
        auth: &Auth<'_, MAX_USER_PROPERTIES>,
    ) -> Result<(), (Self::Error, Option<ReasonCode>)> {
        let State::AwaitServerFinal { server_signature } = &self.state else {
            return Err((ScramError, Some(ReasonCode::ProtocolError)));
        };

        let data = auth
            .authentication_data
            .as_ref()
            .ok_or((ScramError, Some(ReasonCode::ProtocolError)))?;
        let server_final = from_utf8(data.as_bytes())
            .map_err(|_| (ScramError, Some(ReasonCode::ProtocolError)))?;

        debug!("server_final={}", server_final);

        let verifier = server_final
            .strip_prefix("v=")
            .ok_or((ScramError, Some(ReasonCode::ProtocolError)))?;

        if STANDARD
            .decode(verifier)
            .map_err(|_| (ScramError, Some(ReasonCode::ProtocolError)))?
            == *server_signature
        {
            self.state = State::Complete;
            Ok(())
        } else {
            Err((ScramError, Some(ReasonCode::ProtocolError)))
        }
    }
}

fn get_attr<'a>(input: &'a str, tag: &str) -> Result<&'a str, ScramError> {
    input
        .split(',')
        .find_map(|s| s.strip_prefix(tag))
        .ok_or(ScramError)
}

fn hmac(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut h = Hmac::<Sha256>::new_from_slice(key).expect("HMAC size");
    h.update(data);
    h.finalize().into_bytes().into()
}
