use embedded_io_adapters::tokio_1::FromTokio;
use embedded_tls::webpki::CertVerifier;
use embedded_tls::*;
use log::{error, info};
use p256::{
    SecretKey,
    ecdsa::{DerSignature, SigningKey},
};
use pem_parser::pem_to_der;
use rand::rngs::OsRng;
use rust_mqtt::{
    buffer::*,
    client::{
        Client,
        options::{ConnectOptions, DisconnectOptions},
    },
    config::{KeepAlive, SessionExpiryInterval},
};
use signature::SignerMut;
use std::{
    net::{Ipv4Addr, SocketAddr},
    time::{Duration, SystemTime},
};
use tokio::{net::TcpStream, time::sleep};

// Crypto provider implementation from https://github.com/drogue-iot/embedded-tls/blob/71ae455ecba56a05fca4da206532912f7a4716fe/tests/rustpki_test.rs

#[derive(Default)]
struct Provider {
    rng: OsRng,
    verifier: CertVerifier<Aes128GcmSha256, SystemTime, 4096>,
}

impl CryptoProvider for Provider {
    type CipherSuite = Aes128GcmSha256;
    type Signature = DerSignature;

    fn rng(&mut self) -> impl embedded_tls::CryptoRngCore {
        &mut self.rng
    }

    fn verifier(&mut self) -> Result<&mut impl TlsVerifier<Aes128GcmSha256>, TlsError> {
        Ok(&mut self.verifier)
    }

    fn signer(
        &mut self,
        key_der: &[u8],
    ) -> Result<(impl SignerMut<Self::Signature>, SignatureScheme), TlsError> {
        let secret_key =
            SecretKey::from_sec1_der(key_der).map_err(|_| TlsError::InvalidPrivateKey)?;

        Ok((
            SigningKey::from(&secret_key),
            SignatureScheme::EcdsaSecp256r1Sha256,
        ))
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    #[cfg(feature = "alloc")]
    let mut buffer = AllocBuffer;
    #[cfg(feature = "bump")]
    let mut buffer = [0; 1024];
    #[cfg(feature = "bump")]
    let mut buffer = BumpBuffer::new(&mut buffer);

    let mut client = Client::<'_, _, _, 1, 1, 1>::new(&mut buffer);

    let addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8883);
    let connection = TcpStream::connect(addr).await.unwrap();
    let connection = FromTokio::new(connection);

    let ca_cert = pem_to_der(include_str!("./pki/ca-cert.pem"));
    let client_cert = pem_to_der(include_str!("./pki/client-cert.pem"));
    let client_key = pem_to_der(include_str!("./pki/client-key.pem"));

    let config = TlsConfig::new()
        .with_ca(Certificate::X509(&ca_cert))
        .with_cert(Certificate::X509(&client_cert))
        .with_priv_key(&client_key)
        .with_server_name("localhost");

    let mut record_read_buf = [0; 16384];
    let mut record_write_buf = [0; 16384];

    let mut tls_connection =
        TlsConnection::new(connection, &mut record_read_buf, &mut record_write_buf);

    tls_connection
        .open(TlsContext::new(&config, Provider::default()))
        .await
        .expect("error establishing TLS connection");

    match client
        .connect(
            tls_connection,
            &ConnectOptions {
                session_expiry_interval: SessionExpiryInterval::EndOnDisconnect,
                clean_start: true,
                keep_alive: KeepAlive::Infinite,
                will: None,
                user_name: None,
                password: None,
            },
            None,
        )
        .await
    {
        Ok(c) => info!("Connected to server: {:?}", c),
        Err(e) => {
            error!("Failed to connect to server: {:?}", e);
            return;
        }
    };

    sleep(Duration::from_secs(5)).await;

    client
        .disconnect(&DisconnectOptions {
            publish_will: false,
            session_expiry_interval: None,
        })
        .await
        .unwrap();
}
