use std::{
    net::{Ipv4Addr, SocketAddr},
    panic,
    str::from_utf8,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use embedded_io_adapters::tokio_1::FromTokio;
use hmac::{Hmac, KeyInit, Mac};
use log::{debug, error, info};
use pbkdf2::pbkdf2_hmac;
use rand::{Rng, distributions::Alphanumeric};
use rust_mqtt::{
    auth::{AuthMechanism, AuthOptions},
    buffer::*,
    client::{
        Client,
        event::{Auth, Event},
        options::{ConnectOptions, DisconnectOptions, ReAuthOptions},
    },
    types::{MqttBinary, MqttString, ReasonCode},
};
use sha2::{Digest, Sha256};
use tokio::net::TcpStream;
use tokio_test::assert_ok;

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut buffer = AllocBuffer;

    let mut client = Client::<'_, '_, _, _, 1, 1, 1, 1, 16>::new(&mut buffer);

    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 1883);
    let connection = TcpStream::connect(addr).await.unwrap();
    let connection = FromTokio::new(connection);

    // ENHANCED AUTHENTICATION ON CONNECT

    let (mut scram, connect_authentication_data) = ScramSha256::new("test", "testPass");

    match client
        .connect_enhanced(
            connection,
            &ConnectOptions::new()
                .clean_start()
                .authentication_data(connect_authentication_data),
            None,
            MqttString::from_str("SCRAM-SHA-256").unwrap(),
            &mut scram,
        )
        .await
    {
        Ok(c) => {
            info!("Connected to server: {c:?}");
            info!("{:?}", client.client_config());
            info!("{:?}", client.server_config());
            info!("{:?}", client.shared_config());
            info!("{:?}", client.session());
        }
        Err(e) => {
            error!("Failed to connect to server: {e:?}");
            return;
        }
    }

    // (ENHANCED) RE-AUTHENTICATION

    let (mut scram, authentication_data) = ScramSha256::new("test", "testPass");

    assert_ok!(
        client
            .reauthenticate(&ReAuthOptions::new().authentication_data(authentication_data))
            .await
    );

    match assert_ok!(client.poll().await) {
        Event::Auth(auth) if auth.reason_code == ReasonCode::ContinueAuthentication => {
            let Ok(r) = scram.kontinue(&auth) else {
                assert_ok!(
                    client
                        .disconnect(
                            &DisconnectOptions::new().reason_code(ReasonCode::ProtocolError)
                        )
                        .await
                );
                return;
            };

            let mut options = ReAuthOptions::new();
            options.authentication_data = r.authentication_data;

            assert_ok!(client.reauthenticate(&options).await);
        }
        Event::Auth(_) => {
            panic!("server skipped server first data step");
        }
        e => info!("Received event {e:?}"),
    }

    match assert_ok!(client.poll().await) {
        Event::Auth(auth)
            if auth.reason_code == ReasonCode::Success && scram.success(&auth).is_ok() =>
        {
            info!("re-authentication complete");
        }
        Event::Auth(_) => {
            panic!("server didn't complete re-authentication as expected");
        }
        e => info!("Received event {e:?}"),
    }

    match client.disconnect(&DisconnectOptions::new()).await {
        Ok(_n) => {
            // For a correct TCP disconnection, one should make sure the underlying TCP socket
            // sends a FIN segment. However I could not get the tokio::TcpStream to behave that
            // way, so we just do nothing here. It's fine for MQTT operability realistically,
            // but for clean usage, the TCP should be closed properly.

            info!("Disconnected from server")
        }
        Err(e) => {
            error!("Failed to disconnect from server: {e:?}");
        }
    }
}

#[derive(Debug)]
pub enum ScramError {
    ProtocolError(&'static str),
    CryptoError,
    AuthFailed,
}

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
            return Err((
                ScramError::ProtocolError("Invalid state"),
                Some(ReasonCode::ProtocolError),
            ));
        };

        let server_first = auth
            .authentication_data
            .as_ref()
            .and_then(|d| from_utf8(d.as_bytes()).ok())
            .ok_or((
                ScramError::ProtocolError("Invalid server challenge"),
                Some(ReasonCode::ProtocolError),
            ))?;

        let full_nonce =
            get_attr(server_first, "r=").map_err(|e| (e, Some(ReasonCode::ProtocolError)))?;
        let salt = STANDARD
            .decode(get_attr(server_first, "s=").map_err(|e| (e, Some(ReasonCode::ProtocolError)))?)
            .map_err(|_| (ScramError::CryptoError, Some(ReasonCode::ProtocolError)))?;
        let iter = get_attr(server_first, "i=")
            .map_err(|e| (e, Some(ReasonCode::ProtocolError)))?
            .parse::<u32>()
            .map_err(|_| {
                (
                    ScramError::ProtocolError("Invalid iter"),
                    Some(ReasonCode::ProtocolError),
                )
            })?;

        debug!(
            "full_nonce={}, salt={:?}, iterations={}",
            full_nonce, salt, iter
        );

        if !full_nonce.starts_with(client_nonce) {
            error!(
                "server nonce (={}) does not start with client nonce (={})",
                full_nonce, client_nonce
            );

            return Err((
                ScramError::ProtocolError("Invalid state"),
                Some(ReasonCode::ProtocolError),
            ));
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
            return Err((
                ScramError::ProtocolError("Out of order"),
                Some(ReasonCode::ProtocolError),
            ));
        };

        let data = auth.authentication_data.as_ref().ok_or((
            ScramError::ProtocolError("No data"),
            Some(ReasonCode::ProtocolError),
        ))?;
        let server_final = from_utf8(data.as_bytes()).map_err(|_| {
            (
                ScramError::ProtocolError("UTF-8"),
                Some(ReasonCode::ProtocolError),
            )
        })?;

        debug!("server_final={}", server_final);

        let verifier = server_final.strip_prefix("v=").ok_or((
            ScramError::ProtocolError("Invalid verifier format"),
            Some(ReasonCode::ProtocolError),
        ))?;

        if STANDARD
            .decode(verifier)
            .map_err(|_| (ScramError::CryptoError, Some(ReasonCode::ProtocolError)))?
            == *server_signature
        {
            self.state = State::Complete;
            Ok(())
        } else {
            Err((ScramError::AuthFailed, Some(ReasonCode::ProtocolError)))
        }
    }
}

fn get_attr<'a>(input: &'a str, tag: &str) -> Result<&'a str, ScramError> {
    input
        .split(',')
        .find_map(|s| s.strip_prefix(tag))
        .ok_or(ScramError::ProtocolError("Missing attribute"))
}

fn hmac(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut h = Hmac::<Sha256>::new_from_slice(key).expect("HMAC size");
    h.update(data);
    h.finalize().into_bytes().into()
}
